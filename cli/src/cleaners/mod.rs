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
use std::path::PathBuf;

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
#[derive(Debug, Clone, Default)]
pub struct CleanOptions {
    pub dry_run: bool,
    pub force: bool,
    pub verbose: bool,
    pub min_age_days: Option<u64>,
}

/// Trait for cleaners
pub trait Cleaner {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn clean(&self, options: &CleanOptions) -> Result<CleanResult>;
    fn estimate(&self) -> Result<u64>;
}

/// Remove a file or directory
pub fn remove_path(path: &PathBuf, dry_run: bool) -> Result<u64> {
    let size = crate::scanner::get_size(path);

    if dry_run {
        return Ok(size);
    }

    if path.is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }

    Ok(size)
}
