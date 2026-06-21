//! Disk-usage (sunburst) commands.
//!
//! `scan_usage` walks a directory off the async runtime (in `spawn_blocking`,
//! so the UI never freezes), caches the FULL size tree, and returns a pruned
//! payload for the initial sunburst. Drill-in is served from the cache by
//! `get_usage_subtree` with no re-walk. Deletion is Trash-only and routed
//! through the same validated `remove_path` used everywhere else.

use macpurge::{build_usage_tree, UsageNode, UsageScanOptions, UsageScanProgressData};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State};

/// Global cancellation flag for the in-flight usage scan. A new scan resets it;
/// `cancel_usage_scan` sets it. Only one usage scan runs at a time from the UI.
static SCAN_CANCEL: AtomicBool = AtomicBool::new(false);

/// Caches the FULL (un-pruned) usage tree for the most recently scanned root,
/// so drill-in / re-prune never re-walks the disk.
#[derive(Default)]
pub struct UsageCache(pub Mutex<Option<(PathBuf, UsageNode)>>);

/// Initial-payload pruning parameters.
const PAYLOAD_DEPTH: usize = 5;
const PAYLOAD_TOP_N: usize = 30;
const PAYLOAD_MIN_BYTES: u64 = 1 << 20; // 1 MiB
/// Drill-in min-bytes is lower so deeper, smaller items still surface.
const SUBTREE_MIN_BYTES: u64 = 64 << 10; // 64 KiB

/// Scan `path` into a size tree, cache the full tree, and return a pruned payload.
///
/// The walk runs inside `spawn_blocking` so the async runtime (and thus the UI)
/// is never blocked. Progress is emitted on `usage-scan-progress`, throttled to
/// ~10 events/sec, with a final `{ done: true }` event.
#[tauri::command]
pub async fn scan_usage(
    app: AppHandle,
    cache: State<'_, UsageCache>,
    path: String,
    on_disk: bool,
) -> Result<UsageNode, String> {
    let root = PathBuf::from(&path);
    if !root.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    // Reset cancellation for this new scan.
    SCAN_CANCEL.store(false, Ordering::SeqCst);

    let app_for_scan = app.clone();
    let opts = UsageScanOptions {
        on_disk,
        ..Default::default()
    };

    // Walk off the async runtime; never walk on the runtime (would freeze UI).
    let full = tokio::task::spawn_blocking(move || {
        // Throttle progress emits to ~10/sec.
        let mut last_emit = Instant::now() - Duration::from_secs(1);
        let mut progress = |data: UsageScanProgressData| {
            if data.done || last_emit.elapsed() >= Duration::from_millis(100) {
                last_emit = Instant::now();
                let _ = app_for_scan.emit("usage-scan-progress", data);
            }
        };
        build_usage_tree(&root, &opts, &SCAN_CANCEL, &mut progress)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("scan task failed: {}", e))??;

    let payload = full.to_payload(PAYLOAD_DEPTH, PAYLOAD_TOP_N, PAYLOAD_MIN_BYTES);

    // Cache the full tree for drill-in.
    if let Ok(mut guard) = cache.0.lock() {
        *guard = Some((full.path.clone(), full));
    }

    Ok(payload)
}

/// Return a pruned subtree for `path` from the cached full tree (no re-walk).
#[tauri::command]
pub fn get_usage_subtree(
    cache: State<'_, UsageCache>,
    path: String,
) -> Result<UsageNode, String> {
    let target = PathBuf::from(&path);
    let guard = cache
        .0
        .lock()
        .map_err(|_| "usage cache is poisoned".to_string())?;
    let (_, tree) = guard
        .as_ref()
        .ok_or_else(|| "no usage scan cached; run scan_usage first".to_string())?;
    let node = tree
        .find(&target)
        .ok_or_else(|| format!("path not found in cached tree: {}", path))?;
    Ok(node.to_payload(PAYLOAD_DEPTH, PAYLOAD_TOP_N, SUBTREE_MIN_BYTES))
}

/// Request cancellation of the in-flight usage scan.
#[tauri::command]
pub fn cancel_usage_scan() -> bool {
    SCAN_CANCEL.store(true, Ordering::SeqCst);
    true
}

/// Move a usage-tree item to the Trash (reversible). Returns freed bytes.
///
/// Uses the shared, validated `remove_path` (trash-aware, never `remove_dir_all`
/// through a symlink, rejects protected roots). The freed node is pruned from
/// the cached tree so the UI's totals stay consistent without a re-scan.
#[tauri::command]
pub fn trash_usage_item(
    cache: State<'_, UsageCache>,
    path: String,
) -> Result<u64, String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    // SAFE delete only: trash-aware, validated. `remove_path(.., false, true)`
    // moves to Trash and returns the freed size.
    let freed = macpurge::remove_path(&p, false, true).map_err(|e| e.to_string())?;

    // Best-effort: drop the trashed node from the cached tree.
    if let Ok(mut guard) = cache.0.lock() {
        if let Some((_, tree)) = guard.as_mut() {
            prune_node(tree, &p);
        }
    }

    Ok(freed)
}

/// Remove the descendant whose path equals `target` from `node`'s subtree and
/// subtract its size/count from every ancestor on the way down, keeping the
/// cached totals consistent. Returns the removed node's (size, count) if found.
fn prune_node(node: &mut UsageNode, target: &std::path::Path) -> Option<(u64, u64)> {
    // Direct child match.
    if let Some(idx) = node.children.iter().position(|c| c.path == target) {
        let removed = node.children.remove(idx);
        node.size_bytes = node.size_bytes.saturating_sub(removed.size_bytes);
        node.file_count = node.file_count.saturating_sub(removed.file_count);
        return Some((removed.size_bytes, removed.file_count));
    }
    // Otherwise descend into the child that contains the target.
    for child in node.children.iter_mut() {
        if target.starts_with(&child.path) {
            if let Some((size, count)) = prune_node(child, target) {
                node.size_bytes = node.size_bytes.saturating_sub(size);
                node.file_count = node.file_count.saturating_sub(count);
                return Some((size, count));
            }
        }
    }
    None
}

/// Return the user's home directory, used as the default scan root.
#[tauri::command]
pub fn get_home_dir() -> Result<String, String> {
    std::env::var("HOME").map_err(|_| "HOME is not set".to_string())
}

/// Reveal a path in Finder (`open -R <path>`).
#[tauri::command]
pub fn reveal_in_finder(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    std::process::Command::new("open")
        .arg("-R")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("Failed to reveal in Finder: {}", e))?;
    Ok(())
}
