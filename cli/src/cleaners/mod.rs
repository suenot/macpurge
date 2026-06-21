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
pub fn remove_path(path: &Path, dry_run: bool, use_trash: bool) -> Result<u64> {
    let size = crate::scanner::get_size(path);

    if dry_run {
        return Ok(size);
    }

    if use_trash {
        trash::delete(path)
            .map_err(|e| anyhow::anyhow!("Failed to move {} to Trash: {}", path.display(), e))?;
    } else if path.is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }

    Ok(size)
}
