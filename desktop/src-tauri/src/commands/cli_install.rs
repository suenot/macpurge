//! Install the bundled `macpurge` CLI binary onto the user's PATH.

use serde::Serialize;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::{path::BaseDirectory, AppHandle, Manager};

#[derive(Serialize, Default)]
pub struct CliStatus {
    pub installed: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub on_path: bool,
    pub bundled: bool,
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Directories we install to, in order of preference.
fn candidate_dirs() -> Vec<PathBuf> {
    let mut v = vec![PathBuf::from("/usr/local/bin"), PathBuf::from("/opt/homebrew/bin")];
    if let Some(h) = home() {
        v.push(h.join(".local/bin"));
    }
    v
}

fn writable(dir: &Path) -> bool {
    if !dir.is_dir() {
        return false;
    }
    let probe = dir.join(".macpurge-write-test");
    match std::fs::File::create(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

fn login_path() -> Vec<PathBuf> {
    Command::new("/bin/zsh")
        .args(["-lc", "echo $PATH"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().split(':').map(PathBuf::from).collect())
        .unwrap_or_default()
}

fn version_of(bin: &Path) -> Option<String> {
    let out = Command::new(bin).arg("--version").output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn status(app: &AppHandle) -> CliStatus {
    let bundled = app
        .path()
        .resolve("resources/macpurge", BaseDirectory::Resource)
        .map(|p| p.is_file())
        .unwrap_or(false);
    let path_dirs = login_path();
    for dir in candidate_dirs() {
        let bin = dir.join("macpurge");
        if bin.is_file() {
            return CliStatus {
                installed: true,
                version: version_of(&bin),
                on_path: path_dirs.contains(&dir),
                path: Some(bin.to_string_lossy().into_owned()),
                bundled,
            };
        }
    }
    CliStatus { bundled, ..Default::default() }
}

#[tauri::command]
pub fn get_cli_status(app: AppHandle) -> CliStatus {
    status(&app)
}

#[tauri::command]
pub fn install_cli(app: AppHandle) -> Result<CliStatus, String> {
    let src = app
        .path()
        .resolve("resources/macpurge", BaseDirectory::Resource)
        .map_err(|e| e.to_string())?;
    if !src.is_file() {
        return Err("This build does not include the CLI binary. Install it with: cargo install --git https://github.com/suenot/macpurge macpurge".into());
    }

    let path_dirs = login_path();
    // Prefer a writable directory that is already on PATH, then any writable one.
    let mut dirs = candidate_dirs();
    dirs.sort_by_key(|d| !(path_dirs.contains(d) && (writable(d) || d.ends_with(".local/bin"))));
    let target_dir = dirs
        .into_iter()
        .find(|d| writable(d) || d.ends_with(".local/bin"))
        .ok_or("No writable install directory found")?;

    std::fs::create_dir_all(&target_dir).map_err(|e| format!("{}: {e}", target_dir.display()))?;
    let dest = target_dir.join("macpurge");
    let tmp = target_dir.join(".macpurge.tmp");
    std::fs::copy(&src, &tmp).map_err(|e| format!("copy failed: {e}"))?;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    let _ = Command::new("xattr").args(["-d", "com.apple.quarantine"]).arg(&tmp).output();
    std::fs::rename(&tmp, &dest).map_err(|e| format!("install failed: {e}"))?;

    Ok(status(&app))
}
