pub mod dev_artifacts;
pub mod system_cache;
pub mod package_managers;
pub mod applications;
pub mod devtools;
pub mod ai_models;

pub use dev_artifacts::*;
pub use system_cache::*;
pub use package_managers::*;
pub use applications::*;
pub use devtools::*;
pub use ai_models::*;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Result of a cleaning operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanResult {
    pub items_removed: usize,
    pub bytes_freed: u64,
    pub errors: Vec<String>,
    pub removed_paths: Vec<PathBuf>,
}

impl CleanResult {
    pub fn new() -> Self {
        Self {
            items_removed: 0,
            bytes_freed: 0,
            errors: Vec::new(),
            removed_paths: Vec::new(),
        }
    }

    pub fn bytes_freed_human(&self) -> String {
        crate::utils::format_size(self.bytes_freed)
    }

    pub fn merge(&mut self, other: CleanResult) {
        self.items_removed += other.items_removed;
        self.bytes_freed += other.bytes_freed;
        self.errors.extend(other.errors);
        self.removed_paths.extend(other.removed_paths);
    }
}

impl Default for CleanResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Options for cleaning operations
#[derive(Debug, Clone)]
pub struct CleanOptions {
    pub dry_run: bool,
    pub force: bool,
    pub verbose: bool,
    pub min_age_days: Option<u64>,
    /// Move deleted paths to the macOS Trash (reversible) instead of permanently deleting them.
    pub use_trash: bool,
}

impl Default for CleanOptions {
    fn default() -> Self {
        Self {
            dry_run: false,
            force: false,
            verbose: false,
            min_age_days: None,
            // Default to the safe, reversible behavior so callers that don't
            // explicitly opt out never hard-delete user data.
            use_trash: true,
        }
    }
}

/// Trait for cleaners
pub trait Cleaner {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn clean(&self, options: &CleanOptions) -> Result<CleanResult>;
    fn estimate(&self) -> Result<u64>;
}

/// Remove a file or directory.
///
/// When `dry_run` is set nothing is deleted and only the size is returned.
/// When `use_trash` is set the path is moved to the macOS Trash (reversible)
/// via the `trash` crate; otherwise it is permanently deleted.
///
/// Symlinks are handled explicitly and are **never** followed for the purpose of
/// recursive deletion: if `path` itself is a symlink we remove only the link,
/// never `remove_dir_all` through it into the (possibly unrelated, possibly
/// protected) target directory.
pub fn remove_path(path: &Path, dry_run: bool, use_trash: bool) -> Result<u64> {
    // Inspect the path *without* following symlinks so we can decide how to
    // delete it. `symlink_metadata` reports the link itself rather than its
    // target. If it fails (e.g. the path no longer exists) we fall back to the
    // generic file-or-directory handling below.
    let link_meta = std::fs::symlink_metadata(path);
    let is_symlink = link_meta
        .as_ref()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);

    // Size accounting. For a symlink we deliberately do NOT use `get_size`,
    // which follows the link and would attribute the target's bytes to a
    // deletion that only removes the link. Report the link's own size instead
    // (typically a handful of bytes). Size is secondary to delete correctness.
    let size = if is_symlink {
        link_meta.as_ref().map(|m| m.len()).unwrap_or(0)
    } else {
        crate::scanner::get_size(path)
    };

    if dry_run {
        return Ok(size);
    }

    // Defense in depth: never delete a protected location, even if a caller or
    // the frontend supplied a crafted or symlinked path. Validation resolves the
    // real (symlink-followed) path and rejects protected roots; on failure we
    // return an error and delete nothing.
    crate::utils::validate_delete_target(path)?;

    if use_trash {
        // `trash::delete` on a symlink removes only the link, not its target
        // (it does not recurse through the link), so this is safe for symlinks,
        // regular files, and real directories alike.
        trash::delete(path)
            .map_err(|e| anyhow::anyhow!("Failed to move {} to Trash: {}", path.display(), e))?;
    } else if is_symlink {
        // The path itself is a symlink: remove ONLY the link. `remove_file`
        // unlinks the symlink and never touches the directory or file it points
        // at, whereas `remove_dir_all` would recurse through the link.
        std::fs::remove_file(path)?;
    } else if path.is_dir() {
        // A real directory (not a symlink): recurse as before.
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }

    Ok(size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    /// Removing a symlink that points at a directory must delete only the link,
    /// leaving the real target directory and its contents untouched. This is the
    /// regression test for GitLab #12 (remove_path must not recurse through a
    /// symlink via `remove_dir_all`).
    #[test]
    fn test_remove_path_symlink_does_not_follow_into_target() {
        let tmp = tempfile::tempdir().unwrap();

        // Real directory with a file inside it.
        let real_dir = tmp.path().join("real_dir");
        std::fs::create_dir(&real_dir).unwrap();
        let real_file = real_dir.join("keep.txt");
        std::fs::write(&real_file, b"important data").unwrap();

        // Symlink pointing at the real directory.
        let link = tmp.path().join("link_to_dir");
        symlink(&real_dir, &link).unwrap();
        assert!(std::fs::symlink_metadata(&link).unwrap().file_type().is_symlink());

        // Permanently delete the symlink (use_trash = false, dry_run = false).
        remove_path(&link, false, false).unwrap();

        // The symlink itself is gone...
        assert!(
            std::fs::symlink_metadata(&link).is_err(),
            "the symlink should have been removed"
        );
        // ...but the real target directory and its file must still exist.
        assert!(real_dir.is_dir(), "the real target directory must survive");
        assert!(real_file.is_file(), "the file inside the target must survive");
        assert_eq!(std::fs::read(&real_file).unwrap(), b"important data");
    }
}
