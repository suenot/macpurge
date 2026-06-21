//! Disk-usage size tree (sunburst backend).
//!
//! A single-pass [`WalkDir`] walk builds an in-memory [`UsageNode`] tree whose
//! every directory carries its recursive on-disk size and file count. The full
//! tree is built once (and cached by the caller); the UI never receives the
//! whole thing — [`UsageNode::to_payload`] returns a depth-capped, top-N-pruned
//! copy with the long tail folded into a single synthetic bucket so totals still
//! add up. Drill-in is served from the cached tree via [`UsageNode::find`].
//!
//! Sizing reuses the same sparse-file-correct accounting as the rest of the
//! scanner: on Unix with `on_disk` we use `metadata.blocks() * 512` (so e.g.
//! `Docker.raw` is counted by what it actually occupies, not its logical len).

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use walkdir::WalkDir;

/// 512-byte block size used by Unix `stat`/`metadata.blocks()`.
const BLOCK_SIZE: u64 = 512;

/// One node in the usage tree.
///
/// When fully populated (in the cached tree) `children` holds every child and
/// `has_more`/`aggregated_count` are unused. After [`UsageNode::to_payload`]
/// the same struct is reused as the wire format: `children` is pruned, and
/// `has_more` / `aggregated_count` describe what was dropped.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageNode {
    /// Basename only (e.g. `node_modules`), not the full path.
    pub name: String,
    /// Absolute path (used by the frontend for trash/reveal).
    pub path: PathBuf,
    /// Recursive on-disk (or logical) size in bytes.
    pub size_bytes: u64,
    /// Recursive file count (regular files only; dirs/symlinks excluded).
    pub file_count: u64,
    pub is_dir: bool,
    pub is_symlink: bool,
    /// Children, sorted descending by size. Empty for a leaf or a pruned node.
    pub children: Vec<UsageNode>,
    /// True when this node has children that were not serialized into
    /// `children` (drill-in must fetch them via [`UsageNode::find`]).
    pub has_more: bool,
    /// `Some(n)` only on the synthetic "<n smaller items>" aggregation bucket.
    pub aggregated_count: Option<u64>,
}

/// Options controlling a usage scan.
#[derive(Debug, Clone)]
pub struct UsageScanOptions {
    /// Maximum directory depth to descend (root is depth 0).
    pub max_depth: usize,
    /// When true, size by on-disk blocks (`blocks * 512`); else logical length.
    pub on_disk: bool,
    /// When true, follow symlinks. Default false: symlinks are marked but never
    /// traversed (avoids loops and double counting).
    pub follow_links: bool,
    /// Globally-excluded directories (already expanded). Any entry equal to or
    /// nested under one of these is pruned from the walk: it is never descended
    /// into and its size is never counted. Empty by default.
    pub excluded: Vec<PathBuf>,
}

impl Default for UsageScanOptions {
    fn default() -> Self {
        Self {
            max_depth: 12,
            on_disk: true,
            follow_links: false,
            excluded: Vec::new(),
        }
    }
}

/// Progress payload emitted periodically during a scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageScanProgressData {
    pub scanned_items: u64,
    pub current_path: String,
    pub bytes_so_far: u64,
    pub unreadable_count: u64,
    pub done: bool,
}

/// Per-file size in bytes, honoring `on_disk` (sparse-file-correct on Unix).
fn file_size(metadata: &std::fs::Metadata, on_disk: bool) -> u64 {
    if on_disk {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            return metadata.blocks() * BLOCK_SIZE;
        }
        #[cfg(not(unix))]
        {
            return metadata.len();
        }
    }
    metadata.len()
}

/// Build the full usage tree rooted at `root`.
///
/// Uses a single `WalkDir` pass with `contents_first(true)`: children are
/// yielded before their parent directory, so by the time we see a directory we
/// have already accumulated the sizes/counts of everything it contains. Symlinks
/// are never traversed (only marked) unless `follow_links` is set, which avoids
/// cycles and double counting. Unreadable entries are skipped but counted.
///
/// Honors cancellation: checks `cancel` every ~1024 entries and returns the
/// (partial) tree built so far when set. `progress` is invoked periodically.
pub fn build_usage_tree(
    root: &Path,
    opts: &UsageScanOptions,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(UsageScanProgressData),
) -> Result<UsageNode> {
    let root = root.to_path_buf();

    // Accumulators keyed by directory path. Each directory's running size/count
    // is rolled up into its parent once the directory itself is visited (which,
    // with contents_first, happens after all its descendants).
    let mut dir_size: HashMap<PathBuf, u64> = HashMap::new();
    let mut dir_count: HashMap<PathBuf, u64> = HashMap::new();
    // Children collected for each directory (already finalized nodes).
    let mut dir_children: HashMap<PathBuf, Vec<UsageNode>> = HashMap::new();

    let mut scanned_items: u64 = 0;
    let mut bytes_so_far: u64 = 0;
    let mut unreadable_count: u64 = 0;
    let mut root_node: Option<UsageNode> = None;

    let walker = WalkDir::new(&root)
        .follow_links(opts.follow_links)
        .max_depth(opts.max_depth)
        .contents_first(true);

    // `filter_entry` cannot be used to prune here: it is incompatible with
    // `contents_first(true)` (rejecting a directory drops its siblings too).
    // Instead we test each yielded entry against the pre-expanded exclude list
    // and skip any excluded entry before accounting. Because every path under an
    // excluded directory also satisfies `is_path_excluded`, the entire excluded
    // subtree is skipped and never rolled into any parent's size/count.
    let excluded = &opts.excluded;
    let mut iter = walker.into_iter();
    loop {
        let next = match iter.next() {
            Some(item) => item,
            None => break,
        };

        scanned_items += 1;

        // Cancellation check every ~1024 entries.
        if scanned_items % 1024 == 0 {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            progress(UsageScanProgressData {
                scanned_items,
                current_path: String::new(),
                bytes_so_far,
                unreadable_count,
                done: false,
            });
        }

        // Skip unreadable entries but count them.
        let entry = match next {
            Ok(e) => e,
            Err(_) => {
                unreadable_count += 1;
                continue;
            }
        };

        let path = entry.path().to_path_buf();

        // Skip globally-excluded paths (and everything nested under them): they
        // are never sized, counted, or rolled into a parent.
        if !excluded.is_empty() && crate::config::is_path_excluded(&path, excluded) {
            continue;
        }

        let file_type = entry.file_type();
        // `symlink_metadata`-equivalent: WalkDir with follow_links(false) reports
        // the link itself, so `file_type.is_symlink()` is the link, not target.
        let is_symlink = file_type.is_symlink();

        if is_symlink {
            // Mark symlinks as leaves with their own (link) size; never traverse.
            let size = entry
                .metadata()
                .map(|m| file_size(&m, opts.on_disk))
                .unwrap_or(0);
            bytes_so_far += size;
            let node = UsageNode {
                name: basename(&path),
                path: path.clone(),
                size_bytes: size,
                file_count: 0,
                is_dir: false,
                is_symlink: true,
                children: Vec::new(),
                has_more: false,
                aggregated_count: None,
            };
            roll_into_parent(
                &path,
                &root,
                node,
                size,
                0,
                &mut dir_size,
                &mut dir_count,
                &mut dir_children,
            );
            continue;
        }

        if file_type.is_dir() {
            // All descendants have been visited (contents_first); finalize.
            let size = *dir_size.get(&path).unwrap_or(&0);
            let count = *dir_count.get(&path).unwrap_or(&0);
            dir_size.remove(&path);
            dir_count.remove(&path);
            let mut children = dir_children.remove(&path).unwrap_or_default();
            children.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

            let node = UsageNode {
                name: basename(&path),
                path: path.clone(),
                size_bytes: size,
                file_count: count,
                is_dir: true,
                is_symlink: false,
                children,
                has_more: false,
                aggregated_count: None,
            };

            if path == root {
                root_node = Some(node);
            } else {
                roll_into_parent(
                    &path,
                    &root,
                    node,
                    size,
                    count,
                    &mut dir_size,
                    &mut dir_count,
                    &mut dir_children,
                );
            }
            continue;
        }

        // Regular file (or other non-dir, non-symlink): count its size.
        if file_type.is_file() {
            let size = entry
                .metadata()
                .map(|m| file_size(&m, opts.on_disk))
                .unwrap_or(0);
            bytes_so_far += size;
            let node = UsageNode {
                name: basename(&path),
                path: path.clone(),
                size_bytes: size,
                file_count: 1,
                is_dir: false,
                is_symlink: false,
                children: Vec::new(),
                has_more: false,
                aggregated_count: None,
            };
            roll_into_parent(
                &path,
                &root,
                node,
                size,
                1,
                &mut dir_size,
                &mut dir_count,
                &mut dir_children,
            );
        }
    }

    // Final progress tick.
    progress(UsageScanProgressData {
        scanned_items,
        current_path: root.display().to_string(),
        bytes_so_far,
        unreadable_count,
        done: true,
    });

    // Normal case: root was a directory and got finalized in the loop.
    if let Some(node) = root_node {
        return Ok(node);
    }

    // Root itself is a file (or a symlink, or did not yield as a dir). Build a
    // single-node tree directly from its metadata.
    let meta = std::fs::symlink_metadata(&root)
        .map_err(|e| anyhow!("cannot stat {}: {}", root.display(), e))?;
    let is_symlink = meta.file_type().is_symlink();
    let size = file_size(&meta, opts.on_disk);
    Ok(UsageNode {
        name: basename(&root),
        path: root.clone(),
        size_bytes: size,
        file_count: if meta.is_file() { 1 } else { 0 },
        is_dir: meta.is_dir(),
        is_symlink,
        children: Vec::new(),
        has_more: false,
        aggregated_count: None,
    })
}

/// Roll a finalized child node (and its size/count) into its parent's
/// accumulators. The parent is the lexical parent directory of `path`; if it is
/// outside `root` (shouldn't happen for non-root entries) the node is dropped.
#[allow(clippy::too_many_arguments)]
fn roll_into_parent(
    path: &Path,
    root: &Path,
    node: UsageNode,
    size: u64,
    count: u64,
    dir_size: &mut HashMap<PathBuf, u64>,
    dir_count: &mut HashMap<PathBuf, u64>,
    dir_children: &mut HashMap<PathBuf, Vec<UsageNode>>,
) {
    if let Some(parent) = path.parent() {
        // Only roll up within the scanned subtree.
        if parent.starts_with(root) || parent == root {
            let parent = parent.to_path_buf();
            *dir_size.entry(parent.clone()).or_insert(0) += size;
            *dir_count.entry(parent.clone()).or_insert(0) += count;
            dir_children.entry(parent).or_default().push(node);
        }
    }
}

/// Basename of a path as an owned `String`, falling back to the full path.
fn basename(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

impl UsageNode {
    /// Return a pruned copy suitable for sending to the UI.
    ///
    /// - Descends at most `depth_cap` levels (the returned root is depth 0).
    /// - At each kept node, retains the largest `top_n` children whose size is
    ///   `>= min_bytes`. The remaining children (the long tail) are folded into a
    ///   single synthetic `"<N smaller items>"` bucket whose `size_bytes` is the
    ///   sum of the folded sizes, so the parent's total is preserved exactly.
    /// - Sets `has_more = true` on any node whose children were capped, folded,
    ///   or dropped by the depth limit.
    pub fn to_payload(&self, depth_cap: usize, top_n: usize, min_bytes: u64) -> UsageNode {
        let mut out = UsageNode {
            name: self.name.clone(),
            path: self.path.clone(),
            size_bytes: self.size_bytes,
            file_count: self.file_count,
            is_dir: self.is_dir,
            is_symlink: self.is_symlink,
            children: Vec::new(),
            has_more: false,
            aggregated_count: self.aggregated_count,
        };

        if self.children.is_empty() {
            return out;
        }

        // Depth limit reached: keep this node as a leaf but flag that children exist.
        if depth_cap == 0 {
            out.has_more = true;
            return out;
        }

        // Children are kept sorted desc by size in the full tree; sort defensively
        // in case this is called on a hand-built node.
        let mut sorted: Vec<&UsageNode> = self.children.iter().collect();
        sorted.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

        let mut kept: Vec<UsageNode> = Vec::new();
        let mut folded_bytes: u64 = 0;
        let mut folded_count: u64 = 0;

        for child in sorted {
            if kept.len() < top_n && child.size_bytes >= min_bytes {
                kept.push(child.to_payload(depth_cap - 1, top_n, min_bytes));
            } else {
                folded_bytes += child.size_bytes;
                folded_count += 1;
            }
        }

        if folded_count > 0 {
            out.has_more = true;
            kept.push(UsageNode {
                name: format!("<{} smaller items>", folded_count),
                // Point at the parent so the frontend can still drill in there.
                path: self.path.clone(),
                size_bytes: folded_bytes,
                file_count: 0,
                is_dir: true,
                is_symlink: false,
                children: Vec::new(),
                has_more: true,
                aggregated_count: Some(folded_count),
            });
        }

        out.children = kept;
        out
    }

    /// Find a node by its absolute path within this (sub)tree.
    ///
    /// Returns the node whose `path` equals `path`, searching this node and all
    /// descendants. Only descends into subtrees that could contain the target.
    pub fn find(&self, path: &Path) -> Option<&UsageNode> {
        if self.path == path {
            return Some(self);
        }
        // Prune: only descend where `path` lives under this node.
        if !path.starts_with(&self.path) {
            return None;
        }
        for child in &self.children {
            if let Some(found) = child.find(path) {
                return Some(found);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::sync::atomic::AtomicBool;

    /// Build a temp tree:
    ///   root/
    ///     a.txt            (4096 logical-ish)
    ///     big/
    ///       b.bin          (large)
    ///     small/
    ///       c.txt          (tiny)
    ///       d.txt          (tiny)
    fn make_tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        let mut a = fs::File::create(root.join("a.txt")).unwrap();
        a.write_all(&vec![b'x'; 4096]).unwrap();

        fs::create_dir(root.join("big")).unwrap();
        let mut b = fs::File::create(root.join("big/b.bin")).unwrap();
        b.write_all(&vec![b'y'; 200_000]).unwrap();

        fs::create_dir(root.join("small")).unwrap();
        let mut c = fs::File::create(root.join("small/c.txt")).unwrap();
        c.write_all(b"hi").unwrap();
        let mut d = fs::File::create(root.join("small/d.txt")).unwrap();
        d.write_all(b"yo").unwrap();

        dir
    }

    fn scan(root: &Path, on_disk: bool) -> UsageNode {
        let cancel = AtomicBool::new(false);
        let opts = UsageScanOptions {
            max_depth: 12,
            on_disk,
            follow_links: false,
            excluded: Vec::new(),
        };
        let mut noop = |_p: UsageScanProgressData| {};
        build_usage_tree(root, &opts, &cancel, &mut noop).unwrap()
    }

    #[test]
    fn sizes_sum_correctly() {
        let dir = make_tree();
        // Use logical sizes for a deterministic assertion independent of the FS
        // block size.
        let tree = scan(dir.path(), false);

        assert!(tree.is_dir);
        assert_eq!(tree.file_count, 4); // a, b, c, d

        // Root size == sum of children sizes.
        let child_sum: u64 = tree.children.iter().map(|c| c.size_bytes).sum();
        assert_eq!(tree.size_bytes, child_sum);

        // Logical total: 4096 + 200000 + 2 + 2.
        assert_eq!(tree.size_bytes, 4096 + 200_000 + 2 + 2);

        // big/ should be the largest child.
        let big = tree.children.iter().find(|c| c.name == "big").unwrap();
        assert_eq!(big.size_bytes, 200_000);
        assert_eq!(big.file_count, 1);

        let small = tree.children.iter().find(|c| c.name == "small").unwrap();
        assert_eq!(small.size_bytes, 4);
        assert_eq!(small.file_count, 2);
    }

    #[test]
    fn children_sorted_desc() {
        let dir = make_tree();
        let tree = scan(dir.path(), false);
        for w in tree.children.windows(2) {
            assert!(w[0].size_bytes >= w[1].size_bytes);
        }
    }

    #[test]
    fn to_payload_caps_depth() {
        let dir = make_tree();
        let tree = scan(dir.path(), false);

        // depth_cap = 1: root's children present, but grandchildren omitted.
        let payload = tree.to_payload(1, 30, 0);
        assert!(!payload.children.is_empty());
        let big = payload.children.iter().find(|c| c.name == "big").unwrap();
        // Children of `big` are at depth 2, beyond the cap.
        assert!(big.children.is_empty());
        assert!(big.has_more, "depth-capped dir must flag has_more");
    }

    #[test]
    fn to_payload_top_n_and_aggregation_preserves_total() {
        let dir = make_tree();
        let tree = scan(dir.path(), false);

        // Keep only the single largest child; the rest fold into one bucket.
        let payload = tree.to_payload(5, 1, 0);

        // One real child (the largest) + one aggregation bucket.
        assert_eq!(payload.children.len(), 2);
        assert!(payload.has_more);

        let bucket = payload
            .children
            .iter()
            .find(|c| c.aggregated_count.is_some())
            .expect("aggregation bucket present");
        // Root had 3 children (a.txt, big, small); the largest (big) is kept and
        // the remaining two fold into the bucket.
        assert_eq!(bucket.aggregated_count, Some(2));
        assert!(bucket.has_more);
        assert!(bucket.name.contains("smaller items"));

        // Crucially: children still sum to the parent's total.
        let child_sum: u64 = payload.children.iter().map(|c| c.size_bytes).sum();
        assert_eq!(child_sum, payload.size_bytes);
    }

    #[test]
    fn to_payload_min_bytes_folds_small() {
        let dir = make_tree();
        let tree = scan(dir.path(), false);

        // min_bytes above `small` (4 bytes) but below `big` (200k): small folds.
        let payload = tree.to_payload(5, 30, 1000);
        let bucket = payload
            .children
            .iter()
            .find(|c| c.aggregated_count.is_some())
            .expect("small child should be folded by min_bytes");
        assert!(bucket.size_bytes >= 4);
        // Total still preserved.
        let child_sum: u64 = payload.children.iter().map(|c| c.size_bytes).sum();
        assert_eq!(child_sum, payload.size_bytes);
    }

    #[test]
    fn find_locates_subtree() {
        let dir = make_tree();
        let tree = scan(dir.path(), false);

        let big_path = dir.path().join("big");
        let found = tree.find(&big_path).expect("find big/");
        assert_eq!(found.name, "big");
        assert_eq!(found.size_bytes, 200_000);

        // A leaf file is findable too.
        let b_path = dir.path().join("big/b.bin");
        let found_file = tree.find(&b_path).expect("find big/b.bin");
        assert_eq!(found_file.file_count, 1);
        assert!(!found_file.is_dir);

        // A non-existent path is not found.
        assert!(tree.find(&dir.path().join("nope")).is_none());
    }

    #[test]
    fn excluded_subdir_is_pruned_and_uncounted() {
        let dir = make_tree();
        let root = dir.path();
        let excluded_subdir = root.join("big");

        let cancel = AtomicBool::new(false);
        let opts = UsageScanOptions {
            max_depth: 12,
            on_disk: false,
            follow_links: false,
            excluded: vec![excluded_subdir.clone()],
        };
        let mut noop = |_p: UsageScanProgressData| {};
        let tree = build_usage_tree(root, &opts, &cancel, &mut noop).unwrap();

        // `big/` (200_000 bytes) is excluded: it must not appear as a child...
        assert!(
            tree.children.iter().all(|c| c.name != "big"),
            "excluded subdir must not appear in the tree"
        );
        // ...nor be findable anywhere in the tree...
        assert!(
            tree.find(&excluded_subdir).is_none(),
            "excluded subdir must not be reachable via find()"
        );
        // ...and its size must not be counted toward the root total. Only
        // a.txt (4096) + small/ (4) remain.
        assert_eq!(tree.size_bytes, 4096 + 2 + 2);
        assert_eq!(tree.file_count, 3); // a, c, d (b.bin excluded)
    }

    #[test]
    fn cancellation_returns_early() {
        let dir = make_tree();
        let cancel = AtomicBool::new(true); // pre-cancelled
        let opts = UsageScanOptions {
            max_depth: 12,
            on_disk: false,
            follow_links: false,
            excluded: Vec::new(),
        };
        let mut noop = |_p: UsageScanProgressData| {};
        // Should not panic and should return some (possibly partial) tree.
        let res = build_usage_tree(dir.path(), &opts, &cancel, &mut noop);
        assert!(res.is_ok());
    }
}