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
    pub status: String, // "cleaning_artifacts", "compressing", "copying", "deleting_source", "completed", "error", "extracting"
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
    tokio::task::spawn_blocking(|| {
        let config = macpurge::load_config().map_err(|e| e.to_string())?;
        let project_dirs = macpurge::get_project_dirs(&config);

        if project_dirs.is_empty() {
            return Ok(Vec::new());
        }

        let excluded = macpurge::get_excluded_dirs(&config);
        let projects = macpurge::ProjectScanner::scan_projects(&project_dirs, &excluded);
        Ok::<_, String>(projects)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// List all projects as a tree structure (groups + standalone projects)
#[tauri::command]
pub async fn list_projects_tree() -> Result<Vec<macpurge::ProjectTreeEntry>, String> {
    tokio::task::spawn_blocking(|| {
        let config = macpurge::load_config().map_err(|e| e.to_string())?;
        let project_dirs = macpurge::get_project_dirs(&config);

        if project_dirs.is_empty() {
            return Ok(Vec::new());
        }

        let excluded = macpurge::get_excluded_dirs(&config);
        let tree = macpurge::ProjectScanner::scan_projects_tree(&project_dirs, &excluded);
        Ok::<_, String>(tree)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Clone, serde::Serialize)]
pub struct GroupSizePayload {
    pub path: String,
    pub size_bytes: u64,
    pub artifact_size: u64,
}

/// Calculate size for a project group folder and emit event
#[tauri::command]
pub async fn calculate_group_size(app: AppHandle, path: String) -> Result<(), String> {
    let app = app.clone();
    tokio::task::spawn_blocking(move || {
        let path_buf = PathBuf::from(&path);
        if !path_buf.exists() {
            return Err("Path does not exist".to_string());
        }

        let (total_size, _) = macpurge::calculate_dir_size(&path_buf);

        // Sum artifact sizes across child projects
        let mut total_artifact_size: u64 = 0;
        if let Ok(entries) = std::fs::read_dir(&path_buf) {
            for entry in entries.flatten() {
                let child_path = entry.path();
                if child_path.is_dir() {
                    let project_type = macpurge::ProjectType::detect(&child_path);
                    for artifact_dir in project_type.artifact_dirs() {
                        let artifact_path = child_path.join(artifact_dir);
                        if artifact_path.exists() {
                            let (size, _) = macpurge::calculate_dir_size(&artifact_path);
                            total_artifact_size += size;
                        }
                    }
                }
            }
        }

        let _ = app.emit("group-size-calculated", GroupSizePayload {
            path,
            size_bytes: total_size,
            artifact_size: total_artifact_size,
        });

        Ok::<_, String>(())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Calculate size for a single project and emit event
#[tauri::command]
pub async fn calculate_project_size(app: AppHandle, path: String) -> Result<(), String> {
    let app = app.clone();
    tokio::task::spawn_blocking(move || {
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

        Ok::<_, String>(())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Archive selected projects to archive_dir with optional artifact cleaning and zip compression
#[tauri::command]
pub async fn archive_projects(
    app: AppHandle,
    paths: Vec<String>,
) -> Result<Vec<String>, String> {
    let app = app.clone();
    tokio::task::spawn_blocking(move || {
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

    let clean_artifacts = config.archive.clean_artifacts_before_archive;
    let compress_zip = config.archive.compress_to_zip;

    let mut archived = Vec::new();

    for path_str in &paths {
        let source = PathBuf::from(path_str);
        let project_name = source
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // Detect project type before cleaning (source still intact)
        let project_type = macpurge::ProjectType::detect(&source);

        // Step 1: Clean dev artifacts if enabled
        if clean_artifacts {
            let _ = app.emit(
                "archive-progress",
                ArchiveProgressPayload {
                    project_name: project_name.clone(),
                    status: "cleaning_artifacts".to_string(),
                    bytes_copied: 0,
                    total_bytes: 0,
                    error: None,
                },
            );
            macpurge::clean_artifacts_from_project(&source);
        }

        // Step 2: Archive (zip or copy)
        if compress_zip {
            let dest = macpurge::unique_zip_dest_path(&archive_dir, &project_name);

            let _ = app.emit(
                "archive-progress",
                ArchiveProgressPayload {
                    project_name: project_name.clone(),
                    status: "compressing".to_string(),
                    bytes_copied: 0,
                    total_bytes: 0,
                    error: None,
                },
            );

            match macpurge::compress_dir_to_zip(&source, &dest) {
                // `compress_dir_to_zip` only returns Ok after re-opening and
                // validating the archive (see `verify_zip`), so reaching this
                // arm means the zip is complete and the source is safe to move.
                Ok(zip_size) => {
                    // Emit: deleting source
                    let _ = app.emit(
                        "archive-progress",
                        ArchiveProgressPayload {
                            project_name: project_name.clone(),
                            status: "deleting_source".to_string(),
                            bytes_copied: zip_size,
                            total_bytes: zip_size,
                            error: None,
                        },
                    );

                    // Move the source to Trash (recoverable) rather than a hard
                    // delete, and only after archive verification succeeded.
                    if let Err(e) = macpurge::remove_path(&source, false, true) {
                        let _ = std::fs::remove_file(&dest);
                        let _ = app.emit(
                            "archive-progress",
                            ArchiveProgressPayload {
                                project_name: project_name.clone(),
                                status: "error".to_string(),
                                bytes_copied: 0,
                                total_bytes: zip_size,
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
                        size_bytes: zip_size,
                        project_type,
                        archive_format: Some("zip".to_string()),
                    });

                    let _ = macpurge::save_archive_manifest(&archive_dir, &manifest);

                    let _ = app.emit(
                        "archive-progress",
                        ArchiveProgressPayload {
                            project_name: project_name.clone(),
                            status: "completed".to_string(),
                            bytes_copied: zip_size,
                            total_bytes: zip_size,
                            error: None,
                        },
                    );

                    archived.push(path_str.clone());
                }
                Err(e) => {
                    let _ = std::fs::remove_file(&dest);
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
        } else {
            // Legacy: copy directory
            let dest = macpurge::unique_dest_path(&archive_dir, &project_name);

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

            match macpurge::copy_dir_recursive(&source, &dest) {
                Ok(bytes) => {
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

                    if let Err(e) = std::fs::remove_dir_all(&source) {
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
                        project_type,
                        archive_format: Some("directory".to_string()),
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
    }

    Ok::<_, String>(archived)
    })
    .await
    .map_err(|e| e.to_string())?
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

/// Restore archived projects back to original location (supports both zip and directory)
#[tauri::command]
pub async fn restore_projects(
    app: AppHandle,
    archived_paths: Vec<String>,
) -> Result<Vec<String>, String> {
    let app = app.clone();
    tokio::task::spawn_blocking(move || {
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
        let is_zip = entry.archive_format.as_deref() == Some("zip")
            || archived_path.extension().map_or(false, |ext| ext == "zip");

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

        if is_zip {
            // Restore from zip archive
            let _ = app.emit(
                "archive-progress",
                ArchiveProgressPayload {
                    project_name: project_name.clone(),
                    status: "extracting".to_string(),
                    bytes_copied: 0,
                    total_bytes: entry.size_bytes,
                    error: None,
                },
            );

            match macpurge::extract_zip_to_dir(&archived_path, dest) {
                Ok(bytes) => {
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

                    if let Err(e) = std::fs::remove_file(&archived_path) {
                        let _ = std::fs::remove_dir_all(dest);
                        let _ = app.emit(
                            "archive-progress",
                            ArchiveProgressPayload {
                                project_name: project_name.clone(),
                                status: "error".to_string(),
                                bytes_copied: 0,
                                total_bytes: bytes,
                                error: Some(format!("Failed to remove archive zip: {}", e)),
                            },
                        );
                        continue;
                    }

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
        } else {
            // Restore from directory (legacy)
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

            match macpurge::copy_dir_recursive(&archived_path, dest) {
                Ok(bytes) => {
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
    }

    Ok::<_, String>(restored)
    })
    .await
    .map_err(|e| e.to_string())?
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

/// Get archive configuration
#[tauri::command]
pub async fn get_archive_config() -> Result<ArchiveConfigResponse, String> {
    let config = macpurge::load_config().map_err(|e| e.to_string())?;
    Ok(ArchiveConfigResponse {
        scan_batch_size: config.archive.scan_batch_size,
        clean_artifacts_before_archive: config.archive.clean_artifacts_before_archive,
        compress_to_zip: config.archive.compress_to_zip,
    })
}

/// Save archive configuration
#[tauri::command]
pub async fn save_archive_config(archive_config: ArchiveConfigInput) -> Result<(), String> {
    let mut config = macpurge::load_config().map_err(|e| e.to_string())?;
    config.archive.scan_batch_size = archive_config.scan_batch_size;
    config.archive.clean_artifacts_before_archive = archive_config.clean_artifacts_before_archive;
    config.archive.compress_to_zip = archive_config.compress_to_zip;
    macpurge::save_config(&config).map_err(|e| e.to_string())
}

#[derive(Clone, serde::Serialize)]
pub struct ArchiveConfigResponse {
    pub scan_batch_size: usize,
    pub clean_artifacts_before_archive: bool,
    pub compress_to_zip: bool,
}

#[derive(Clone, serde::Deserialize)]
pub struct ArchiveConfigInput {
    pub scan_batch_size: usize,
    pub clean_artifacts_before_archive: bool,
    pub compress_to_zip: bool,
}

// ── Project management actions ───────────────────

/// Delete a project directory permanently
#[tauri::command]
pub async fn delete_project(path: String) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        return Err("Path does not exist".to_string());
    }
    if !path_buf.is_dir() {
        return Err("Path is not a directory".to_string());
    }
    // Guard against deleting a protected location (e.g. a crafted or symlinked
    // path from the frontend). Validate before the recursive delete.
    let path_buf = macpurge::validate_delete_target(&path_buf).map_err(|e| e.to_string())?;
    std::fs::remove_dir_all(&path_buf)
        .map_err(|e| format!("Failed to delete project: {}", e))
}

/// Open a project folder in Finder (macOS)
#[tauri::command]
pub async fn open_project_in_finder(path: String) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        return Err("Path does not exist".to_string());
    }
    std::process::Command::new("open")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("Failed to open Finder: {}", e))?;
    Ok(())
}

// ── Git status & actions ────────────────────────

/// Check git status for a project and emit event
#[tauri::command]
pub async fn check_git_status(app: AppHandle, path: String) -> Result<(), String> {
    let app = app.clone();
    tokio::task::spawn_blocking(move || {
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

        Ok::<_, String>(())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Auto-commit all changes in a project
#[tauri::command]
pub async fn auto_commit_project(path: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let path_buf = PathBuf::from(&path);
        macpurge::ProjectScanner::auto_commit(&path_buf).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Push to a specific remote
#[tauri::command]
pub async fn push_project(path: String, remote: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let path_buf = PathBuf::from(&path);
        macpurge::ProjectScanner::push_to_remote(&path_buf, &remote).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Push to all remotes
#[tauri::command]
pub async fn push_project_all(path: String) -> Result<Vec<PushRemoteResult>, String> {
    tokio::task::spawn_blocking(move || {
        let path_buf = PathBuf::from(&path);
        let results = macpurge::ProjectScanner::push_to_all_remotes(&path_buf)
            .map_err(|e| e.to_string())?;

        Ok::<_, String>(results
            .into_iter()
            .map(|(remote, result)| PushRemoteResult {
                remote,
                success: result.is_ok(),
                error: result.err(),
            })
            .collect())
    })
    .await
    .map_err(|e| e.to_string())?
}
