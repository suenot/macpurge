//! Cleanup commands with real-time progress events

use macpurge::CleanResult;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle, Emitter};

// Per-operation cancellation.
//
// A single shared `AtomicBool` cannot distinguish between concurrent/overlapping
// cleanups: cancelling one would cancel all of them, and starting a new cleanup
// would silently un-cancel a running one. Instead we hand each cleanup a unique
// generation id.
//
// `ACTIVE_GEN` is bumped (and read) every time a cleanup starts; the value after
// the bump is that operation's id. `CANCELLED_GEN` holds the id of the operation
// that has been asked to cancel. A cleanup with id `my_gen` is cancelled iff
// `CANCELLED_GEN == my_gen`. Cancellation therefore targets one specific
// operation, and incrementing `ACTIVE_GEN` for a new cleanup never disturbs the
// cancellation state of a different, already-running one.
//
// Ids start at 1 (the first cleanup gets 1), so 0 is reserved for "no operation".
static ACTIVE_GEN: AtomicU64 = AtomicU64::new(0);
static CANCELLED_GEN: AtomicU64 = AtomicU64::new(0);

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

/// Request cancellation of the most recently started cleanup.
///
/// Targets the operation identified by the current `ACTIVE_GEN`, so it only
/// affects that cleanup and leaves any other concurrent cleanup running.
#[tauri::command]
pub fn cancel_cleanup() -> bool {
    CANCELLED_GEN.store(ACTIVE_GEN.load(Ordering::SeqCst), Ordering::SeqCst);
    true
}

/// Check whether the most recently started cleanup has been cancelled.
///
/// True only when an operation exists (`ACTIVE_GEN != 0`) and it is the one
/// currently marked cancelled.
#[tauri::command]
pub fn is_cleanup_cancelled() -> bool {
    let active = ACTIVE_GEN.load(Ordering::SeqCst);
    active != 0 && CANCELLED_GEN.load(Ordering::SeqCst) == active
}

/// Clean multiple artifact paths with progress events
#[tauri::command]
pub async fn clean_artifacts(
    app: AppHandle,
    paths: Vec<String>,
    dry_run: bool,
) -> Result<CleanResult, String> {
    // Claim a unique generation id for this operation. We do NOT reset any
    // shared flag, so a different cleanup that is already running keeps its own
    // cancellation state. This op is cancelled only when `CANCELLED_GEN` is set
    // to exactly `my_gen`.
    let my_gen = ACTIVE_GEN.fetch_add(1, Ordering::SeqCst) + 1;

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
        // Check for cancellation targeted specifically at this operation.
        if CANCELLED_GEN.load(Ordering::SeqCst) == my_gen {
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
