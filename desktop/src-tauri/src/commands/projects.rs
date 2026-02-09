//! Project management commands with background size scanning

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

// ── Event payloads ───────────────────────────────

#[derive(Clone, serde::Serialize)]
pub struct ProjectSizePayload {
    pub path: String,
    pub size_bytes: u64,
    pub artifact_size: u64,
    pub last_commit_time: Option<i64>,
}

#[derive(Clone, serde::Serialize)]
pub struct GitStatusPayload {
    pub path: String,
    pub status: macpurge::GitStatus,
}

#[derive(Clone, serde::Serialize)]
pub struct PushRemoteResult {
    pub remote: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Clone, serde::Serialize)]
pub struct ArchiveProgressPayload {
    pub project_name: String,
    pub status: String, // "copying", "deleting_source", "completed", "error"
    pub bytes_copied: u64,
    pub total_bytes: u64,
    pub error: Option<String>,
}

#[derive(Clone, serde::Serialize)]
pub struct ArchiveStatusResponse {
    pub configured: bool,
    pub archive_path: Option<String>,
    pub exists: bool,
    pub total_size: u64,
    pub project_count: usize,
}

// ── Commands ─────────────────────────────────────

/// List all projects from configured project directories (fast, no sizes)
#[tauri::command]
pub async fn list_projects() -> Result<Vec<macpurge::ProjectInfo>, String> {
    let config = macpurge::load_config().map_err(|e| e.to_string())?;
    let project_dirs = macpurge::get_project_dirs(&config);

    if project_dirs.is_empty() {
        return Ok(Vec::new());
    }

    let projects = macpurge::ProjectScanner::scan_projects(&project_dirs);
    Ok(projects)
}

/// Calculate size for a single project and emit event
#[tauri::command]
pub async fn calculate_project_size(app: AppHandle, path: String) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        return Err("Path does not exist".to_string());
    }

    let (size, artifact_size) = macpurge::ProjectScanner::calculate_project_size(&path_buf);
    let commit_time = macpurge::ProjectScanner::get_last_commit_time(&path_buf);

    let _ = app.emit("project-size-calculated", ProjectSizePayload {
        path,
        size_bytes: size,
        artifact_size,
        last_commit_time: commit_time,
    });

    Ok(())
}

/// Archive selected projects to archive_dir (copy + delete)
#[tauri::command]
pub async fn archive_projects(
    app: AppHandle,
    paths: Vec<String>,
) -> Result<Vec<String>, String> {
    let config = macpurge::load_config().map_err(|e| e.to_string())?;
    let archive_dir = config
        .archive_dir
        .as_ref()
        .map(|p| macpurge::expand_path(p))
        .ok_or("Archive directory not configured. Set it in Settings.")?;

    if !archive_dir.exists() {
        return Err(format!(
            "Archive directory does not exist: {}",
            archive_dir.display()
        ));
    }

    let mut archived = Vec::new();

    for path_str in &paths {
        let source = PathBuf::from(path_str);
        let project_name = source
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let dest = macpurge::unique_dest_path(&archive_dir, &project_name);

        // Emit: copying
        let _ = app.emit(
            "archive-progress",
            ArchiveProgressPayload {
                project_name: project_name.clone(),
                status: "copying".to_string(),
                bytes_copied: 0,
                total_bytes: 0,
                error: None,
            },
        );

        // Copy recursively
        match macpurge::copy_dir_recursive(&source, &dest) {
            Ok(bytes) => {
                // Emit: deleting source
                let _ = app.emit(
                    "archive-progress",
                    ArchiveProgressPayload {
                        project_name: project_name.clone(),
                        status: "deleting_source".to_string(),
                        bytes_copied: bytes,
                        total_bytes: bytes,
                        error: None,
                    },
                );

                // Delete source
                if let Err(e) = std::fs::remove_dir_all(&source) {
                    // Rollback: remove destination, keep source
                    let _ = std::fs::remove_dir_all(&dest);
                    let _ = app.emit(
                        "archive-progress",
                        ArchiveProgressPayload {
                            project_name: project_name.clone(),
                            status: "error".to_string(),
                            bytes_copied: 0,
                            total_bytes: bytes,
                            error: Some(format!("Failed to remove source: {}", e)),
                        },
                    );
                    continue;
                }

                // Update manifest
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);

                let mut manifest = macpurge::load_archive_manifest(&archive_dir)
                    .unwrap_or(macpurge::ArchiveManifest {
                        version: 1,
                        entries: Vec::new(),
                    });

                manifest.entries.push(macpurge::ArchivedProjectEntry {
                    name: project_name.clone(),
                    original_path: source.clone(),
                    archived_path: dest.clone(),
                    archived_at: now,
                    size_bytes: bytes,
                    project_type: macpurge::ProjectType::detect(&dest),
                });

                let _ = macpurge::save_archive_manifest(&archive_dir, &manifest);

                let _ = app.emit(
                    "archive-progress",
                    ArchiveProgressPayload {
                        project_name: project_name.clone(),
                        status: "completed".to_string(),
                        bytes_copied: bytes,
                        total_bytes: bytes,
                        error: None,
                    },
                );

                archived.push(path_str.clone());
            }
            Err(e) => {
                // Cleanup partial copy
                let _ = std::fs::remove_dir_all(&dest);
                let _ = app.emit(
                    "archive-progress",
                    ArchiveProgressPayload {
                        project_name: project_name.clone(),
                        status: "error".to_string(),
                        bytes_copied: 0,
                        total_bytes: 0,
                        error: Some(e.to_string()),
                    },
                );
            }
        }
    }

    Ok(archived)
}

/// List archived projects from archive_dir manifest
#[tauri::command]
pub async fn list_archived_projects() -> Result<Vec<macpurge::ArchivedProjectEntry>, String> {
    let config = macpurge::load_config().map_err(|e| e.to_string())?;
    let archive_dir = match config.archive_dir.as_ref() {
        Some(p) => macpurge::expand_path(p),
        None => return Ok(Vec::new()),
    };

    if !archive_dir.exists() {
        return Ok(Vec::new());
    }

    let manifest = macpurge::load_archive_manifest(&archive_dir)
        .unwrap_or_default();

    // Filter to only entries whose archived_path still exists
    let existing: Vec<_> = manifest
        .entries
        .into_iter()
        .filter(|e| e.archived_path.exists())
        .collect();

    Ok(existing)
}

/// Restore archived projects back to original location
#[tauri::command]
pub async fn restore_projects(
    app: AppHandle,
    archived_paths: Vec<String>,
) -> Result<Vec<String>, String> {
    let config = macpurge::load_config().map_err(|e| e.to_string())?;
    let archive_dir = config
        .archive_dir
        .as_ref()
        .map(|p| macpurge::expand_path(p))
        .ok_or("Archive directory not configured")?;

    if !archive_dir.exists() {
        return Err("Archive directory not found".to_string());
    }

    let mut manifest = macpurge::load_archive_manifest(&archive_dir)
        .unwrap_or_default();

    let mut restored = Vec::new();

    for archived_path_str in &archived_paths {
        let archived_path = PathBuf::from(archived_path_str);

        // Find the manifest entry for this path
        let entry = manifest
            .entries
            .iter()
            .find(|e| e.archived_path == archived_path)
            .cloned();

        let entry = match entry {
            Some(e) => e,
            None => continue,
        };

        let dest = &entry.original_path;
        let project_name = entry.name.clone();

        // Check if original location already exists
        if dest.exists() {
            let _ = app.emit(
                "archive-progress",
                ArchiveProgressPayload {
                    project_name: project_name.clone(),
                    status: "error".to_string(),
                    bytes_copied: 0,
                    total_bytes: entry.size_bytes,
                    error: Some(format!(
                        "Original location already exists: {}",
                        dest.display()
                    )),
                },
            );
            continue;
        }

        // Ensure parent directory exists
        if let Some(parent) = dest.parent() {
            if !parent.exists() {
                let _ = std::fs::create_dir_all(parent);
            }
        }

        // Emit: copying
        let _ = app.emit(
            "archive-progress",
            ArchiveProgressPayload {
                project_name: project_name.clone(),
                status: "copying".to_string(),
                bytes_copied: 0,
                total_bytes: entry.size_bytes,
                error: None,
            },
        );

        // Copy from archive to original location
        match macpurge::copy_dir_recursive(&archived_path, dest) {
            Ok(bytes) => {
                // Delete archived copy
                let _ = app.emit(
                    "archive-progress",
                    ArchiveProgressPayload {
                        project_name: project_name.clone(),
                        status: "deleting_source".to_string(),
                        bytes_copied: bytes,
                        total_bytes: bytes,
                        error: None,
                    },
                );

                if let Err(e) = std::fs::remove_dir_all(&archived_path) {
                    let _ = std::fs::remove_dir_all(dest);
                    let _ = app.emit(
                        "archive-progress",
                        ArchiveProgressPayload {
                            project_name: project_name.clone(),
                            status: "error".to_string(),
                            bytes_copied: 0,
                            total_bytes: bytes,
                            error: Some(format!("Failed to remove archive copy: {}", e)),
                        },
                    );
                    continue;
                }

                // Remove from manifest
                manifest
                    .entries
                    .retain(|e| e.archived_path != archived_path);
                let _ = macpurge::save_archive_manifest(&archive_dir, &manifest);

                let _ = app.emit(
                    "archive-progress",
                    ArchiveProgressPayload {
                        project_name: project_name.clone(),
                        status: "completed".to_string(),
                        bytes_copied: bytes,
                        total_bytes: bytes,
                        error: None,
                    },
                );

                restored.push(archived_path_str.clone());
            }
            Err(e) => {
                let _ = std::fs::remove_dir_all(dest);
                let _ = app.emit(
                    "archive-progress",
                    ArchiveProgressPayload {
                        project_name: project_name.clone(),
                        status: "error".to_string(),
                        bytes_copied: 0,
                        total_bytes: 0,
                        error: Some(e.to_string()),
                    },
                );
            }
        }
    }

    Ok(restored)
}

/// Get archive directory status
#[tauri::command]
pub async fn get_archive_status() -> Result<ArchiveStatusResponse, String> {
    let config = macpurge::load_config().map_err(|e| e.to_string())?;

    let archive_path = config
        .archive_dir
        .as_ref()
        .map(|p| macpurge::expand_path(p));

    let (exists, total_size, project_count) = match &archive_path {
        Some(p) if p.exists() => {
            let manifest = macpurge::load_archive_manifest(p).unwrap_or_default();
            let total: u64 = manifest.entries.iter().map(|e| e.size_bytes).sum();
            let count = manifest
                .entries
                .iter()
                .filter(|e| e.archived_path.exists())
                .count();
            (true, total, count)
        }
        _ => (false, 0, 0),
    };

    Ok(ArchiveStatusResponse {
        configured: config.archive_dir.is_some(),
        archive_path: archive_path.map(|p| p.display().to_string()),
        exists,
        total_size,
        project_count,
    })
}

// ── Git status & actions ────────────────────────

/// Check git status for a project and emit event
#[tauri::command]
pub async fn check_git_status(app: AppHandle, path: String) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        return Err("Path does not exist".to_string());
    }

    if let Some(status) = macpurge::ProjectScanner::get_git_status(&path_buf) {
        let _ = app.emit("project-git-status", GitStatusPayload {
            path,
            status,
        });
    }

    Ok(())
}

/// Auto-commit all changes in a project
#[tauri::command]
pub async fn auto_commit_project(path: String) -> Result<String, String> {
    let path_buf = PathBuf::from(&path);
    macpurge::ProjectScanner::auto_commit(&path_buf).map_err(|e| e.to_string())
}

/// Push to a specific remote
#[tauri::command]
pub async fn push_project(path: String, remote: String) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    macpurge::ProjectScanner::push_to_remote(&path_buf, &remote).map_err(|e| e.to_string())
}

/// Push to all remotes
#[tauri::command]
pub async fn push_project_all(path: String) -> Result<Vec<PushRemoteResult>, String> {
    let path_buf = PathBuf::from(&path);
    let results = macpurge::ProjectScanner::push_to_all_remotes(&path_buf)
        .map_err(|e| e.to_string())?;

    Ok(results
        .into_iter()
        .map(|(remote, result)| PushRemoteResult {
            remote,
            success: result.is_ok(),
            error: result.err(),
        })
        .collect())
}
