//! macOS permission (TCC) helpers: Full Disk Access detection + onboarding.
//!
//! macOS does not let an app pre-grant or suppress TCC prompts, so the strategy
//! is: detect Full Disk Access (which, once granted in System Settings, silences
//! every per-folder prompt) and, separately, proactively touch the standard
//! protected folders at first launch so their prompts appear up front rather than
//! mid-scan.

use std::path::{Path, PathBuf};

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Best-effort, prompt-free check for Full Disk Access.
///
/// Reads a TCC-protected database that is only openable with Full Disk Access.
/// PermissionDenied => not granted; Ok => granted; anything else is inconclusive
/// and we fall through to the next probe.
#[tauri::command]
pub fn check_full_disk_access() -> bool {
    fn probe(path: &Path) -> Option<bool> {
        match std::fs::File::open(path) {
            Ok(_) => Some(true),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => Some(false),
            Err(_) => None, // not found / other — inconclusive
        }
    }

    if let Some(home) = home_dir() {
        if let Some(r) = probe(&home.join("Library/Application Support/com.apple.TCC/TCC.db")) {
            return r;
        }
        // Secondary FDA-only probe.
        if let Some(r) = probe(&home.join("Library/Safari/CloudTabs.db")) {
            return r;
        }
    }
    if let Some(r) = probe(Path::new("/Library/Application Support/com.apple.TCC/TCC.db")) {
        return r;
    }
    false
}

/// Open System Settings directly on the Full Disk Access pane.
#[tauri::command]
pub fn open_full_disk_access_settings() -> Result<(), String> {
    std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles")
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to open System Settings: {}", e))
}

#[derive(serde::Serialize)]
pub struct FolderAccess {
    pub name: String,
    pub path: String,
    pub accessible: bool,
}

/// Proactively touch the standard protected folders (Downloads/Desktop/Documents)
/// so macOS shows their access prompts now, during onboarding, instead of mid-scan.
///
/// Runs on a blocking thread because reading a protected folder blocks until the
/// user answers the TCC prompt. Caller should invoke this only on an explicit
/// user action.
#[tauri::command]
pub async fn request_standard_folders() -> Result<Vec<FolderAccess>, String> {
    tokio::task::spawn_blocking(|| {
        let home = home_dir().ok_or_else(|| "No home directory".to_string())?;
        let mut out = Vec::new();
        for name in ["Downloads", "Desktop", "Documents"] {
            let p = home.join(name);
            // read_dir triggers the TCC prompt the first time and blocks until answered.
            let accessible = std::fs::read_dir(&p).is_ok();
            out.push(FolderAccess {
                name: name.to_string(),
                path: p.display().to_string(),
                accessible,
            });
        }
        Ok(out)
    })
    .await
    .map_err(|e| e.to_string())?
}
