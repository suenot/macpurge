//! Cleanup commands with real-time progress events

use macpurge::CleanResult;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};

// Global cancellation flag
static CANCEL_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Event payloads
#[derive(Clone, serde::Serialize)]
pub struct CleanStartedPayload {
    pub total_items: usize,
    pub paths: Vec<String>,
}

#[derive(Clone, serde::Serialize)]
pub struct CleanProgressPayload {
    pub current_index: usize,
    pub total_items: usize,
    pub current_path: String,
    pub status: String, // "deleting", "deleted", "error", "skipped"
    pub bytes_freed: u64,
    pub error: Option<String>,
}

#[derive(Clone, serde::Serialize)]
pub struct CleanCompletedPayload {
    pub items_removed: usize,
    pub bytes_freed: u64,
    pub errors: Vec<String>,
    pub cancelled: bool,
}

/// Request cancellation of ongoing cleanup
#[tauri::command]
pub fn cancel_cleanup() -> bool {
    CANCEL_REQUESTED.store(true, Ordering::SeqCst);
    true
}

/// Check if cancellation was requested
#[tauri::command]
pub fn is_cleanup_cancelled() -> bool {
    CANCEL_REQUESTED.load(Ordering::SeqCst)
}

/// Clean multiple artifact paths with progress events
#[tauri::command]
pub async fn clean_artifacts(
    app: AppHandle,
    paths: Vec<String>,
    dry_run: bool,
) -> Result<CleanResult, String> {
    // Reset cancellation flag
    CANCEL_REQUESTED.store(false, Ordering::SeqCst);

    let total_items = paths.len();

    // Emit start event
    let _ = app.emit(
        "clean-started",
        CleanStartedPayload {
            total_items,
            paths: paths.clone(),
        },
    );

    let mut total_result = CleanResult::default();

    // Honor the configured deletion mode (move to Trash vs. permanent delete).
    // Default to the safe, reversible behavior if the config can't be read.
    let use_trash = macpurge::load_config()
        .map(|c| c.cleanup.use_trash)
        .unwrap_or(true);

    for (index, path_str) in paths.iter().enumerate() {
        // Check for cancellation
        if CANCEL_REQUESTED.load(Ordering::SeqCst) {
            let _ = app.emit(
                "clean-completed",
                CleanCompletedPayload {
                    items_removed: total_result.items_removed,
                    bytes_freed: total_result.bytes_freed,
                    errors: total_result.errors.iter().map(|s| s.to_string()).collect(),
                    cancelled: true,
                },
            );
            return Ok(total_result);
        }

        let path = PathBuf::from(&path_str);

        // Emit "deleting" status
        let _ = app.emit(
            "clean-progress",
            CleanProgressPayload {
                current_index: index,
                total_items,
                current_path: path_str.clone(),
                status: "deleting".to_string(),
                bytes_freed: 0,
                error: None,
            },
        );

        if !path.exists() {
            let _ = app.emit(
                "clean-progress",
                CleanProgressPayload {
                    current_index: index,
                    total_items,
                    current_path: path_str.clone(),
                    status: "skipped".to_string(),
                    bytes_freed: 0,
                    error: Some("Path does not exist".to_string()),
                },
            );
            continue;
        }

        // Guard against deleting a protected location (e.g. a crafted or
        // symlinked path from the frontend). Validate before any deletion.
        if let Err(e) = macpurge::validate_delete_target(&path) {
            let error_msg = format!("{}: {}", path_str, e);
            total_result.errors.push(error_msg);
            let _ = app.emit(
                "clean-progress",
                CleanProgressPayload {
                    current_index: index,
                    total_items,
                    current_path: path_str.clone(),
                    status: "error".to_string(),
                    bytes_freed: 0,
                    error: Some(e.to_string()),
                },
            );
            continue;
        }

        match macpurge::remove_path(&path, dry_run, use_trash) {
            Ok(bytes) => {
                total_result.items_removed += 1;
                total_result.bytes_freed += bytes;
                total_result.removed_paths.push(path);

                let _ = app.emit(
                    "clean-progress",
                    CleanProgressPayload {
                        current_index: index,
                        total_items,
                        current_path: path_str.clone(),
                        status: "deleted".to_string(),
                        bytes_freed: bytes,
                        error: None,
                    },
                );
            }
            Err(e) => {
                let error_msg = format!("{}: {}", path_str, e);
                total_result.errors.push(error_msg.clone());

                let _ = app.emit(
                    "clean-progress",
                    CleanProgressPayload {
                        current_index: index,
                        total_items,
                        current_path: path_str.clone(),
                        status: "error".to_string(),
                        bytes_freed: 0,
                        error: Some(e.to_string()),
                    },
                );
            }
        }

        // Small delay for UI responsiveness
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    // Emit completion event
    let _ = app.emit(
        "clean-completed",
        CleanCompletedPayload {
            items_removed: total_result.items_removed,
            bytes_freed: total_result.bytes_freed,
            errors: total_result.errors.iter().map(|s| s.to_string()).collect(),
            cancelled: false,
        },
    );

    Ok(total_result)
}

/// Clean a single path
#[tauri::command]
pub async fn clean_path(app: AppHandle, path: String, dry_run: bool) -> Result<CleanResult, String> {
    clean_artifacts(app, vec![path], dry_run).await
}

/// Estimate how much space can be freed
#[tauri::command]
pub async fn estimate_cleanup(paths: Vec<String>) -> Result<CleanupEstimate, String> {
    tokio::task::spawn_blocking(move || {
        let mut total_bytes: u64 = 0;
        let mut item_count: usize = 0;
        let mut by_type: std::collections::HashMap<String, u64> = std::collections::HashMap::new();

        for path_str in paths {
            let path = PathBuf::from(&path_str);
            if !path.exists() {
                continue;
            }

            let size = macpurge::get_size(&path);
            total_bytes += size;
            item_count += 1;

            // Categorize by directory name
            if let Some(name) = path.file_name() {
                let key = name.to_string_lossy().to_string();
                *by_type.entry(key).or_insert(0) += size;
            }
        }

        Ok::<_, String>(CleanupEstimate {
            total_bytes,
            total_human: format_size(total_bytes),
            item_count,
            by_type: by_type
                .into_iter()
                .map(|(k, v)| TypeEstimate {
                    artifact_type: k,
                    bytes: v,
                    human: format_size(v),
                })
                .collect(),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[derive(serde::Serialize)]
pub struct CleanupEstimate {
    pub total_bytes: u64,
    pub total_human: String,
    pub item_count: usize,
    pub by_type: Vec<TypeEstimate>,
}

#[derive(serde::Serialize)]
pub struct TypeEstimate {
    pub artifact_type: String,
    pub bytes: u64,
    pub human: String,
}
