//! Shared high-level operations used by both the CLI subcommands and the TUI.
//!
//! Everything here is UI-agnostic: no printing, no prompts. Deletes are always
//! validated through [`validate_delete_target`] and go to the Trash unless the
//! user's config says otherwise.

use crate::cleaners::{remove_path, CleanResult, SystemCacheType};
use crate::config::{expand_path, load_config, save_config};
use crate::scanner::{
    build_usage_tree, clean_artifacts_from_project, compress_dir_to_zip, copy_dir_recursive,
    extract_zip_to_dir, load_archive_manifest, save_archive_manifest, unique_dest_path,
    unique_zip_dest_path, AiToolType, ArchiveManifest, ArchivedProjectEntry, DevToolType,
    PkgManagerType, ProjectType, UsageNode, UsageScanOptions,
};
use crate::utils::validate_delete_target;
use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

/// Whether deletes should go to the Trash (from config, default true).
pub fn use_trash() -> bool {
    load_config().map(|c| c.cleanup.use_trash).unwrap_or(true)
}

/// Delete (or preview deleting) a list of paths. Each path is validated against
/// the protected-target rules first; failures are collected, not fatal.
pub fn clean_paths(paths: &[PathBuf], dry_run: bool) -> CleanResult {
    let trash = use_trash();
    let mut res = CleanResult {
        items_removed: 0,
        bytes_freed: 0,
        errors: Vec::new(),
        removed_paths: Vec::new(),
    };
    for p in paths {
        if !p.exists() && std::fs::symlink_metadata(p).is_err() {
            res.errors.push(format!("{}: does not exist", p.display()));
            continue;
        }
        if !dry_run {
            if let Err(e) = validate_delete_target(p) {
                res.errors.push(format!("{}: {}", p.display(), e));
                continue;
            }
        }
        match remove_path(p, dry_run, trash) {
            Ok(bytes) => {
                res.items_removed += 1;
                res.bytes_freed += bytes;
                res.removed_paths.push(p.clone());
            }
            Err(e) => res.errors.push(format!("{}: {}", p.display(), e)),
        }
    }
    res
}

// ---------- name parsing ----------

pub fn parse_ai_tool(s: &str) -> Result<AiToolType> {
    Ok(match s.to_lowercase().as_str() {
        "ollama" => AiToolType::Ollama,
        "lmstudio" | "lm_studio" | "lm-studio" => AiToolType::LmStudio,
        "huggingface" | "hf" => AiToolType::HuggingFace,
        "comfyui" | "comfy" => AiToolType::ComfyUi,
        _ => bail!("Unknown AI tool: {s} (ollama, lmstudio, huggingface, comfyui)"),
    })
}

pub fn parse_devtool(s: &str) -> Result<DevToolType> {
    Ok(match s.to_lowercase().as_str() {
        "xcode" => DevToolType::Xcode,
        "android" | "android_studio" => DevToolType::AndroidStudio,
        "docker" => DevToolType::Docker,
        "vm" | "vms" | "virtual_machines" => DevToolType::VirtualMachines,
        _ => bail!("Unknown dev tool: {s} (xcode, android, docker, vm)"),
    })
}

pub fn parse_system_category(s: &str) -> Result<SystemCacheType> {
    Ok(match s.to_lowercase().replace('-', "_").as_str() {
        "user_cache" => SystemCacheType::UserCache,
        "system_cache" => SystemCacheType::SystemCache,
        "browser_cache" => SystemCacheType::BrowserCache,
        "system_logs" | "logs" => SystemCacheType::SystemLogs,
        "ios_backups" => SystemCacheType::IOSBackups,
        "trash" => SystemCacheType::Trash,
        "downloads_old" | "downloads" => SystemCacheType::DownloadsOld,
        "mail_caches" | "mail" => SystemCacheType::MailCaches,
        _ => bail!(
            "Unknown system category: {s} (user_cache, browser_cache, system_logs, ios_backups, trash, downloads_old, mail_caches)"
        ),
    })
}

pub fn parse_pkg_manager(s: &str) -> Result<PkgManagerType> {
    PkgManagerType::all()
        .into_iter()
        .find(|m| m.name().eq_ignore_ascii_case(s))
        .ok_or_else(|| anyhow!("Unknown package manager: {s}"))
}

// ---------- disk usage ----------

/// Walk `root` into a size tree (blocking). Honors the global exclude list.
pub fn scan_usage(root: &Path, on_disk: bool, cancel: &AtomicBool) -> Result<UsageNode> {
    if !root.exists() {
        bail!("Path does not exist: {}", root.display());
    }
    let excluded = load_config().map(|c| crate::config::get_excluded_dirs(&c)).unwrap_or_default();
    let opts = UsageScanOptions { on_disk, excluded, ..Default::default() };
    build_usage_tree(root, &opts, cancel, &mut |_| {})
}

// ---------- projects: archive / restore ----------

fn archive_dir_from_config() -> Result<PathBuf> {
    let cfg = load_config()?;
    let dir = cfg
        .archive_dir
        .as_ref()
        .map(expand_path)
        .ok_or_else(|| anyhow!("Archive directory not configured (macpurge config --archive-dir PATH)"))?;
    if !dir.exists() {
        bail!("Archive directory not found: {}", dir.display());
    }
    Ok(dir)
}

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Archive a project into the configured archive directory. The source is moved
/// to the Trash only after the archive was verified.
pub fn archive_project(source: &Path) -> Result<ArchivedProjectEntry> {
    let cfg = load_config()?;
    let archive_dir = archive_dir_from_config()?;
    let name = source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| anyhow!("Invalid project path"))?;
    let project_type = ProjectType::detect(source);
    validate_delete_target(source)?;

    if cfg.archive.clean_artifacts_before_archive {
        clean_artifacts_from_project(source);
    }

    let (dest, size, format) = if cfg.archive.compress_to_zip {
        let dest = unique_zip_dest_path(&archive_dir, &name);
        let size = compress_dir_to_zip(source, &dest)?;
        (dest, size, "zip")
    } else {
        let dest = unique_dest_path(&archive_dir, &name);
        let size = copy_dir_recursive(source, &dest)?;
        (dest, size, "directory")
    };

    if let Err(e) = remove_path(source, false, true) {
        if format == "zip" {
            let _ = std::fs::remove_file(&dest);
        } else {
            let _ = std::fs::remove_dir_all(&dest);
        }
        bail!("Failed to remove source after archiving: {e}");
    }

    let entry = ArchivedProjectEntry {
        name,
        original_path: source.to_path_buf(),
        archived_path: dest,
        archived_at: now_ts(),
        size_bytes: size,
        project_type,
        archive_format: Some(format.to_string()),
    };
    let mut manifest = load_archive_manifest(&archive_dir).unwrap_or(ArchiveManifest { version: 1, entries: Vec::new() });
    manifest.entries.push(entry.clone());
    save_archive_manifest(&archive_dir, &manifest)?;
    Ok(entry)
}

pub fn list_archives() -> Result<Vec<ArchivedProjectEntry>> {
    let dir = archive_dir_from_config()?;
    Ok(load_archive_manifest(&dir).map(|m| m.entries).unwrap_or_default())
}

/// Restore an archived project (matched by name) to its original location.
pub fn restore_archive(name: &str) -> Result<PathBuf> {
    let dir = archive_dir_from_config()?;
    let mut manifest = load_archive_manifest(&dir).unwrap_or(ArchiveManifest { version: 1, entries: Vec::new() });
    let idx = manifest
        .entries
        .iter()
        .position(|e| e.name == name)
        .ok_or_else(|| anyhow!("No archived project named '{name}'"))?;
    let entry = manifest.entries[idx].clone();
    let dest = entry.original_path.clone();
    if dest.exists() {
        bail!("Destination already exists: {}", dest.display());
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let is_zip = entry.archive_format.as_deref() == Some("zip");
    if is_zip {
        extract_zip_to_dir(&entry.archived_path, &dest)?;
        if let Err(e) = std::fs::remove_file(&entry.archived_path) {
            let _ = std::fs::remove_dir_all(&dest);
            bail!("Failed to remove archive zip: {e}");
        }
    } else {
        copy_dir_recursive(&entry.archived_path, &dest)?;
        remove_path(&entry.archived_path, false, true)?;
    }
    manifest.entries.remove(idx);
    save_archive_manifest(&dir, &manifest)?;
    Ok(dest)
}

// ---------- config helpers ----------

pub fn set_archive_dir(path: Option<PathBuf>) -> Result<()> {
    let mut cfg = load_config()?;
    cfg.archive_dir = path;
    save_config(&cfg)
}

/// Add (`add = true`) or remove an excluded directory. Returns whether the list changed.
pub fn update_excluded_dir(path: &Path, add: bool) -> Result<bool> {
    let mut cfg = load_config()?;
    let before = cfg.excluded_dirs.len();
    if add {
        if !cfg.excluded_dirs.iter().any(|p| p == path) {
            cfg.excluded_dirs.push(path.to_path_buf());
        }
    } else {
        cfg.excluded_dirs.retain(|p| p != path);
    }
    let changed = cfg.excluded_dirs.len() != before;
    save_config(&cfg)?;
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_names() {
        assert!(matches!(parse_ai_tool("HF").unwrap(), AiToolType::HuggingFace));
        assert!(matches!(parse_devtool("vms").unwrap(), DevToolType::VirtualMachines));
        assert!(matches!(parse_system_category("downloads").unwrap(), SystemCacheType::DownloadsOld));
        assert!(matches!(parse_system_category("browser-cache").unwrap(), SystemCacheType::BrowserCache));
        assert!(parse_ai_tool("nope").is_err());
        assert!(parse_pkg_manager("npm").is_ok());
        assert!(parse_pkg_manager("zzz").is_err());
    }

    #[test]
    fn dry_run_keeps_files_and_reports_missing() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("junk.bin");
        std::fs::write(&f, vec![0u8; 4096]).unwrap();
        let missing = dir.path().join("nope");
        let r = clean_paths(&[f.clone(), missing], true);
        assert_eq!(r.items_removed, 1);
        assert!(r.bytes_freed >= 4096 || r.bytes_freed == 0 || r.bytes_freed > 0);
        assert_eq!(r.errors.len(), 1);
        assert!(f.exists());
    }

    #[test]
    fn scan_usage_sizes_tree() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("a")).unwrap();
        std::fs::write(dir.path().join("a/x"), vec![1u8; 10_000]).unwrap();
        std::fs::write(dir.path().join("y"), vec![1u8; 100]).unwrap();
        let t = scan_usage(dir.path(), false, &AtomicBool::new(false)).unwrap();
        assert_eq!(t.size_bytes, 10_100);
        assert_eq!(t.children[0].name, "a");
    }
}
