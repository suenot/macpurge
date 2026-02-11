//! Project scanner module
//!
//! Scans configured project directories for projects, detects project types,
//! analyzes VCS status, and supports archiving/restoring projects.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read as IoRead, Write as IoWrite};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::UNIX_EPOCH;

use crate::scanner::directory::calculate_dir_size;

/// Type of project detected by marker files
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProjectType {
    Node,
    Rust,
    Go,
    Python,
    Java,
    DotNet,
    Ruby,
    Php,
    Elixir,
    Haskell,
    Swift,
    Dart,
    Unknown,
}

impl ProjectType {
    /// Detect project type by checking for marker files in the root directory
    pub fn detect(project_root: &Path) -> Self {
        if project_root.join("Cargo.toml").exists() {
            ProjectType::Rust
        } else if project_root.join("package.json").exists() {
            ProjectType::Node
        } else if project_root.join("go.mod").exists() {
            ProjectType::Go
        } else if project_root.join("pyproject.toml").exists()
            || project_root.join("setup.py").exists()
            || project_root.join("requirements.txt").exists()
        {
            ProjectType::Python
        } else if project_root.join("pom.xml").exists()
            || project_root.join("build.gradle").exists()
            || project_root.join("build.gradle.kts").exists()
        {
            ProjectType::Java
        } else if has_extension_in_dir(project_root, &["csproj", "fsproj", "sln"]) {
            ProjectType::DotNet
        } else if project_root.join("Gemfile").exists() {
            ProjectType::Ruby
        } else if project_root.join("composer.json").exists() {
            ProjectType::Php
        } else if project_root.join("mix.exs").exists() {
            ProjectType::Elixir
        } else if project_root.join("stack.yaml").exists()
            || has_extension_in_dir(project_root, &["cabal"])
        {
            ProjectType::Haskell
        } else if project_root.join("Package.swift").exists()
            || has_extension_in_dir(project_root, &["xcodeproj"])
        {
            ProjectType::Swift
        } else if project_root.join("pubspec.yaml").exists() {
            ProjectType::Dart
        } else {
            ProjectType::Unknown
        }
    }

    /// Human-readable display name
    pub fn display_name(&self) -> &'static str {
        match self {
            ProjectType::Node => "Node.js",
            ProjectType::Rust => "Rust",
            ProjectType::Go => "Go",
            ProjectType::Python => "Python",
            ProjectType::Java => "Java",
            ProjectType::DotNet => ".NET",
            ProjectType::Ruby => "Ruby",
            ProjectType::Php => "PHP",
            ProjectType::Elixir => "Elixir",
            ProjectType::Haskell => "Haskell",
            ProjectType::Swift => "Swift",
            ProjectType::Dart => "Dart",
            ProjectType::Unknown => "Unknown",
        }
    }

    /// Known artifact directory names for this project type
    pub fn artifact_dirs(&self) -> &[&str] {
        match self {
            ProjectType::Node => &["node_modules", ".next", ".nuxt", "dist", ".output"],
            ProjectType::Rust => &["target"],
            ProjectType::Go => &["vendor"],
            ProjectType::Python => &["venv", ".venv", "__pycache__", ".tox", "dist", "*.egg-info"],
            ProjectType::Java => &["target", "build", ".gradle"],
            ProjectType::DotNet => &["bin", "obj"],
            ProjectType::Ruby => &["vendor/bundle"],
            ProjectType::Php => &["vendor"],
            ProjectType::Elixir => &["_build", "deps"],
            ProjectType::Haskell => &[".stack-work"],
            ProjectType::Swift => &[".build", "DerivedData"],
            ProjectType::Dart => &[".dart_tool", "build"],
            ProjectType::Unknown => &[],
        }
    }
}

/// Information about a single project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    /// Display name (directory name)
    pub name: String,
    /// Full path to the project root
    pub path: PathBuf,
    /// Total project size in bytes (None = not yet calculated)
    pub size_bytes: Option<u64>,
    /// Last modification time (Unix timestamp)
    pub last_modified: Option<i64>,
    /// Detected project type
    pub project_type: ProjectType,
    /// Whether .git directory exists
    pub has_git: bool,
    /// Last git commit timestamp (Unix timestamp)
    pub last_commit_time: Option<i64>,
    /// Size of cleanable artifacts (None = not yet calculated)
    pub artifact_size: Option<u64>,
}

/// Git remote status (ahead/behind per remote)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitRemoteStatus {
    pub name: String,
    pub url: String,
    pub ahead: u32,
    pub behind: u32,
}

/// Git status for a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    pub has_uncommitted: bool,
    pub uncommitted_count: u32,
    pub current_branch: Option<String>,
    pub remotes: Vec<GitRemoteStatus>,
}

/// Archived project manifest entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivedProjectEntry {
    pub name: String,
    pub original_path: PathBuf,
    pub archived_path: PathBuf,
    pub archived_at: i64,
    pub size_bytes: u64,
    pub project_type: ProjectType,
    /// Archive format: "zip" or "directory". None = legacy (directory).
    #[serde(default)]
    pub archive_format: Option<String>,
}

/// The manifest file stored in the archive directory
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArchiveManifest {
    pub version: u32,
    pub entries: Vec<ArchivedProjectEntry>,
}

/// Project scanner for enumerating and analyzing projects
pub struct ProjectScanner;

impl ProjectScanner {
    /// Scan project directories for projects (fast, no size calculation).
    /// Checks 2 levels deep: immediate children + children of non-project folders.
    /// Returns projects sorted by last_modified descending.
    pub fn scan_projects(project_dirs: &[PathBuf]) -> Vec<ProjectInfo> {
        let mut projects = Vec::new();

        for dir in project_dirs {
            if !dir.exists() || !dir.is_dir() {
                continue;
            }

            let entries = match fs::read_dir(dir) {
                Ok(entries) => entries,
                Err(_) => continue,
            };

            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                // Skip hidden directories
                let name = match path.file_name().and_then(|n| n.to_str()) {
                    Some(n) if !n.starts_with('.') => n.to_string(),
                    _ => continue,
                };

                let project_type = ProjectType::detect(&path);
                let has_git = path.join(".git").exists();

                if project_type != ProjectType::Unknown || has_git {
                    // Depth-1: this is a project
                    projects.push(make_project_info(name, path, project_type, has_git));
                } else {
                    // Depth-2: check children of this grouping folder
                    let sub_entries = match fs::read_dir(&path) {
                        Ok(e) => e,
                        Err(_) => continue,
                    };
                    for sub_entry in sub_entries.flatten() {
                        let sub_path = sub_entry.path();
                        if !sub_path.is_dir() {
                            continue;
                        }
                        let sub_name = match sub_path.file_name().and_then(|n| n.to_str()) {
                            Some(n) if !n.starts_with('.') => n.to_string(),
                            _ => continue,
                        };
                        let sub_type = ProjectType::detect(&sub_path);
                        let sub_git = sub_path.join(".git").exists();
                        if sub_type != ProjectType::Unknown || sub_git {
                            projects.push(make_project_info(sub_name, sub_path, sub_type, sub_git));
                        }
                    }
                }
            }
        }

        // Sort by last_modified descending (most recent first)
        projects.sort_by(|a, b| {
            let a_time = a.last_modified.unwrap_or(0);
            let b_time = b.last_modified.unwrap_or(0);
            b_time.cmp(&a_time)
        });

        projects
    }

    /// Calculate size for a single project.
    /// Returns (total_size, artifact_size).
    pub fn calculate_project_size(path: &Path) -> (u64, u64) {
        let (total_size, _) = calculate_dir_size(path);

        let project_type = ProjectType::detect(path);
        let mut artifact_size: u64 = 0;

        for artifact_dir in project_type.artifact_dirs() {
            let artifact_path = path.join(artifact_dir);
            if artifact_path.exists() {
                let (size, _) = calculate_dir_size(&artifact_path);
                artifact_size += size;
            }
        }

        (total_size, artifact_size)
    }

    /// Get last git commit time via `git log -1 --format=%ct`
    pub fn get_last_commit_time(project_path: &Path) -> Option<i64> {
        if !project_path.join(".git").exists() {
            return None;
        }

        let output = Command::new("git")
            .args(["-C", &project_path.to_string_lossy(), "log", "-1", "--format=%ct"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.trim().parse::<i64>().ok()
    }

    /// Get detailed git status: uncommitted changes, branch, remotes with ahead/behind
    pub fn get_git_status(project_path: &Path) -> Option<GitStatus> {
        if !project_path.join(".git").exists() {
            return None;
        }

        let path_str = project_path.to_string_lossy();

        // git status --porcelain
        let uncommitted_count = Command::new("git")
            .args(["-C", &path_str, "status", "--porcelain"])
            .output()
            .ok()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .filter(|l| !l.is_empty())
                    .count() as u32
            })
            .unwrap_or(0);

        // git branch --show-current
        let current_branch = Command::new("git")
            .args(["-C", &path_str, "branch", "--show-current"])
            .output()
            .ok()
            .and_then(|o| {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if s.is_empty() { None } else { Some(s) }
            });

        // git remote -v (parse fetch lines)
        let remotes_output = Command::new("git")
            .args(["-C", &path_str, "remote", "-v"])
            .output()
            .ok();

        let mut remotes = Vec::new();
        let mut seen_remotes = std::collections::HashSet::new();

        if let Some(output) = remotes_output {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                // format: "origin\tgit@github.com:user/repo.git (fetch)"
                if !line.contains("(fetch)") {
                    continue;
                }
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 2 {
                    continue;
                }
                let remote_name = parts[0].to_string();
                if !seen_remotes.insert(remote_name.clone()) {
                    continue;
                }
                let url = parts[1].to_string();

                // Calculate ahead/behind for this remote + current branch
                let (ahead, behind) = if let Some(ref branch) = current_branch {
                    let remote_ref = format!("{}/{}", remote_name, branch);
                    let ahead = git_rev_count(&path_str, &format!("{}..HEAD", remote_ref));
                    let behind = git_rev_count(&path_str, &format!("HEAD..{}", remote_ref));
                    (ahead, behind)
                } else {
                    (0, 0)
                };

                remotes.push(GitRemoteStatus {
                    name: remote_name,
                    url,
                    ahead,
                    behind,
                });
            }
        }

        Some(GitStatus {
            has_uncommitted: uncommitted_count > 0,
            uncommitted_count,
            current_branch,
            remotes,
        })
    }

    /// Auto-commit all changes with a standard message.
    /// Returns the commit hash on success.
    pub fn auto_commit(project_path: &Path) -> anyhow::Result<String> {
        let path_str = project_path.to_string_lossy();

        // git add -A
        let add_output = Command::new("git")
            .args(["-C", &path_str, "add", "-A"])
            .output()?;
        if !add_output.status.success() {
            anyhow::bail!(
                "git add failed: {}",
                String::from_utf8_lossy(&add_output.stderr)
            );
        }

        // git commit -m "macpurge auto commit"
        let commit_output = Command::new("git")
            .args(["-C", &path_str, "commit", "-m", "macpurge auto commit"])
            .output()?;
        if !commit_output.status.success() {
            let stderr = String::from_utf8_lossy(&commit_output.stderr);
            anyhow::bail!("git commit failed: {}", stderr);
        }

        // Get the commit hash
        let hash_output = Command::new("git")
            .args(["-C", &path_str, "rev-parse", "--short", "HEAD"])
            .output()?;
        let hash = String::from_utf8_lossy(&hash_output.stdout).trim().to_string();

        Ok(hash)
    }

    /// Push current branch to a specific remote
    pub fn push_to_remote(project_path: &Path, remote: &str) -> anyhow::Result<()> {
        let path_str = project_path.to_string_lossy();

        // Get current branch
        let branch_output = Command::new("git")
            .args(["-C", &path_str, "branch", "--show-current"])
            .output()?;
        let branch = String::from_utf8_lossy(&branch_output.stdout).trim().to_string();
        if branch.is_empty() {
            anyhow::bail!("Not on a branch (detached HEAD?)");
        }

        let push_output = Command::new("git")
            .args(["-C", &path_str, "push", remote, &branch])
            .output()?;
        if !push_output.status.success() {
            let stderr = String::from_utf8_lossy(&push_output.stderr);
            anyhow::bail!("git push {} failed: {}", remote, stderr);
        }

        Ok(())
    }

    /// Push current branch to all remotes.
    /// Returns a list of (remote_name, result) for each push attempt.
    pub fn push_to_all_remotes(project_path: &Path) -> anyhow::Result<Vec<(String, Result<(), String>)>> {
        let path_str = project_path.to_string_lossy();

        // List remotes
        let remote_output = Command::new("git")
            .args(["-C", &path_str, "remote"])
            .output()?;
        let remote_text = String::from_utf8_lossy(&remote_output.stdout);
        let remotes: Vec<&str> = remote_text.lines().filter(|l| !l.is_empty()).collect();

        if remotes.is_empty() {
            anyhow::bail!("No remotes configured");
        }

        let mut results = Vec::new();
        for remote in remotes {
            match Self::push_to_remote(project_path, remote) {
                Ok(()) => results.push((remote.to_string(), Ok(()))),
                Err(e) => results.push((remote.to_string(), Err(e.to_string()))),
            }
        }

        Ok(results)
    }
}

/// Load archive manifest from archive directory
pub fn load_archive_manifest(archive_dir: &Path) -> Option<ArchiveManifest> {
    let manifest_path = archive_dir.join(".macpurge-archive.json");
    if !manifest_path.exists() {
        return None;
    }

    let content = fs::read_to_string(&manifest_path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Save archive manifest to archive directory
pub fn save_archive_manifest(archive_dir: &Path, manifest: &ArchiveManifest) -> anyhow::Result<()> {
    let manifest_path = archive_dir.join(".macpurge-archive.json");
    let content = serde_json::to_string_pretty(manifest)?;
    fs::write(&manifest_path, content)?;
    Ok(())
}

/// Copy a directory recursively, returns total bytes copied
pub fn copy_dir_recursive(src: &Path, dest: &Path) -> anyhow::Result<u64> {
    fs::create_dir_all(dest)?;
    let mut total_bytes: u64 = 0;

    for entry in walkdir::WalkDir::new(src).min_depth(1) {
        let entry = entry?;
        let relative = entry.path().strip_prefix(src)?;
        let target = dest.join(relative);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            let bytes = fs::copy(entry.path(), &target)?;
            total_bytes += bytes;
        } else if entry.file_type().is_symlink() {
            let link_target = fs::read_link(entry.path())?;
            #[cfg(unix)]
            std::os::unix::fs::symlink(&link_target, &target)?;
        }
    }

    Ok(total_bytes)
}

/// Generate a unique destination path by appending _2, _3, etc. if needed
pub fn unique_dest_path(archive_dir: &Path, name: &str) -> PathBuf {
    let mut dest = archive_dir.join(name);
    if !dest.exists() {
        return dest;
    }

    let mut counter = 2u32;
    loop {
        dest = archive_dir.join(format!("{}_{}", name, counter));
        if !dest.exists() {
            return dest;
        }
        counter += 1;
    }
}

/// Create a ProjectInfo from scanned directory data
fn make_project_info(name: String, path: PathBuf, project_type: ProjectType, has_git: bool) -> ProjectInfo {
    let last_modified = get_dir_mtime(&path);
    ProjectInfo {
        name,
        path,
        size_bytes: None,
        last_modified,
        project_type,
        has_git,
        last_commit_time: None,
        artifact_size: None,
    }
}

// ── Helper functions ─────────────────────────────

/// Count commits in a git revision range (e.g., "origin/main..HEAD")
fn git_rev_count(path_str: &str, range: &str) -> u32 {
    Command::new("git")
        .args(["-C", path_str, "rev-list", "--count", range])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8_lossy(&o.stdout).trim().parse().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}

/// Check if any file in a directory has one of the given extensions
fn has_extension_in_dir(dir: &Path, extensions: &[&str]) -> bool {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return false,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if extensions.contains(&ext) {
                return true;
            }
        }
        // Also check directory names (e.g., .xcodeproj)
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            for wanted_ext in extensions {
                if name.ends_with(&format!(".{}", wanted_ext)) {
                    return true;
                }
            }
        }
    }

    false
}

/// Get modification time of a directory (most recent file within depth=1)
fn get_dir_mtime(path: &Path) -> Option<i64> {
    let mut latest: Option<i64> = None;

    // First check the directory itself
    if let Ok(meta) = fs::metadata(path) {
        if let Ok(modified) = meta.modified() {
            if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                latest = Some(duration.as_secs() as i64);
            }
        }
    }

    // Then check immediate children for a more recent mtime
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                        let ts = duration.as_secs() as i64;
                        if latest.map_or(true, |l| ts > l) {
                            latest = Some(ts);
                        }
                    }
                }
            }
        }
    }

    latest
}

/// Compress a directory into a .zip file, returns total bytes written
pub fn compress_dir_to_zip(source: &Path, dest_zip: &Path) -> anyhow::Result<u64> {
    let file = fs::File::create(dest_zip)?;
    let mut zip_writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for entry in walkdir::WalkDir::new(source).min_depth(1) {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(source)?;
        let name = relative.to_string_lossy();

        if entry.file_type().is_dir() {
            zip_writer.add_directory(format!("{}/", name), options)?;
        } else if entry.file_type().is_file() {
            zip_writer.start_file(name.to_string(), options)?;
            let mut f = fs::File::open(path)?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer)?;
            zip_writer.write_all(&buffer)?;
        }
        // Symlinks are skipped in zip archives
    }

    zip_writer.finish()?;
    // Return the size of the zip file itself
    let zip_size = fs::metadata(dest_zip)?.len();
    Ok(zip_size)
}

/// Extract a .zip file to a directory, returns total bytes extracted
pub fn extract_zip_to_dir(zip_path: &Path, dest: &Path) -> anyhow::Result<u64> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut total_bytes: u64 = 0;

    fs::create_dir_all(dest)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        let outpath = dest.join(&name);

        if entry.is_dir() {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut outfile = fs::File::create(&outpath)?;
            let bytes = std::io::copy(&mut entry, &mut outfile)?;
            total_bytes += bytes;

            // Restore permissions on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = entry.unix_mode() {
                    fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
                }
            }
        }
    }

    Ok(total_bytes)
}

/// Remove known dev artifact directories from a project.
/// Returns total bytes removed.
pub fn clean_artifacts_from_project(project_path: &Path) -> u64 {
    let project_type = ProjectType::detect(project_path);
    let mut bytes_removed: u64 = 0;

    for artifact_dir in project_type.artifact_dirs() {
        let artifact_path = project_path.join(artifact_dir);
        if artifact_path.exists() && artifact_path.is_dir() {
            let (size, _) = calculate_dir_size(&artifact_path);
            if fs::remove_dir_all(&artifact_path).is_ok() {
                bytes_removed += size;
            }
        }
    }

    bytes_removed
}

/// Generate a unique .zip destination path by appending _2, _3, etc. if needed
pub fn unique_zip_dest_path(archive_dir: &Path, name: &str) -> PathBuf {
    let mut dest = archive_dir.join(format!("{}.zip", name));
    if !dest.exists() {
        return dest;
    }

    let mut counter = 2u32;
    loop {
        dest = archive_dir.join(format!("{}_{}.zip", name, counter));
        if !dest.exists() {
            return dest;
        }
        counter += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_detect_node_project() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        assert_eq!(ProjectType::detect(dir.path()), ProjectType::Node);
    }

    #[test]
    fn test_detect_rust_project() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "").unwrap();
        assert_eq!(ProjectType::detect(dir.path()), ProjectType::Rust);
    }

    #[test]
    fn test_detect_unknown_project() {
        let dir = tempdir().unwrap();
        assert_eq!(ProjectType::detect(dir.path()), ProjectType::Unknown);
    }

    #[test]
    fn test_unique_dest_path() {
        let dir = tempdir().unwrap();
        let name = "myproject";

        // First call: no conflict
        let dest = unique_dest_path(dir.path(), name);
        assert_eq!(dest, dir.path().join("myproject"));

        // Create the directory
        fs::create_dir(&dest).unwrap();

        // Second call: should get _2
        let dest2 = unique_dest_path(dir.path(), name);
        assert_eq!(dest2, dir.path().join("myproject_2"));
    }

    #[test]
    fn test_scan_projects_empty() {
        let dir = tempdir().unwrap();
        let projects = ProjectScanner::scan_projects(&[dir.path().to_path_buf()]);
        assert!(projects.is_empty());
    }

    #[test]
    fn test_scan_projects_with_node() {
        let dir = tempdir().unwrap();
        let project_dir = dir.path().join("my-app");
        fs::create_dir(&project_dir).unwrap();
        fs::write(project_dir.join("package.json"), "{}").unwrap();

        let projects = ProjectScanner::scan_projects(&[dir.path().to_path_buf()]);
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "my-app");
        assert_eq!(projects[0].project_type, ProjectType::Node);
    }
}
