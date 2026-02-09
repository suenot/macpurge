use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use walkdir::WalkDir;

use super::file_types::FileType;

/// Represents a scanned directory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedEntry {
    pub path: PathBuf,
    pub file_type: FileType,
    pub size_bytes: u64,
    pub file_count: u64,
    pub last_modified: Option<i64>,
}

impl ScannedEntry {
    pub fn size_human(&self) -> String {
        crate::utils::format_size(self.size_bytes)
    }
}

/// Result of a directory scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub root_path: PathBuf,
    pub entries: Vec<ScannedEntry>,
    pub total_size: u64,
    pub total_files: u64,
    pub scan_duration_ms: u64,
    pub by_type: HashMap<String, Vec<ScannedEntry>>,
}

impl ScanResult {
    pub fn total_size_human(&self) -> String {
        crate::utils::format_size(self.total_size)
    }

    /// Get entries sorted by size (largest first)
    pub fn sorted_by_size(&self) -> Vec<&ScannedEntry> {
        let mut entries: Vec<&ScannedEntry> = self.entries.iter().collect();
        entries.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
        entries
    }

    /// Get top N entries by size
    pub fn top_n(&self, n: usize) -> Vec<&ScannedEntry> {
        self.sorted_by_size().into_iter().take(n).collect()
    }
}

/// Options for scanning
#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    pub dev_mode: bool,
    pub include_hidden: bool,
    pub max_depth: Option<usize>,
    pub min_size: Option<u64>,
    pub show_progress: bool,
}

/// Scanner for directories
pub struct DirectoryScanner {
    options: ScanOptions,
}

impl DirectoryScanner {
    pub fn new(options: ScanOptions) -> Self {
        Self { options }
    }

    /// Scan a directory for dev artifacts
    pub fn scan(&self, root: &Path) -> Result<ScanResult> {
        let start = std::time::Instant::now();
        let mut entries = Vec::new();
        let mut by_type: HashMap<String, Vec<ScannedEntry>> = HashMap::new();

        let progress = if self.options.show_progress {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} [{elapsed_precise}] {msg}")
                    .unwrap(),
            );
            pb.set_message("Scanning...");
            Some(pb)
        } else {
            None
        };

        let patterns = if self.options.dev_mode {
            super::file_types::DEV_ARTIFACT_PATTERNS.to_vec()
        } else {
            vec![
                ("node_modules", FileType::NodeModules),
                (".venv", FileType::PythonVenv),
                ("venv", FileType::PythonVenv),
                ("target", FileType::RustTarget),
            ]
        };

        let mut walker = WalkDir::new(root).follow_links(false);

        if let Some(max_depth) = self.options.max_depth {
            walker = walker.max_depth(max_depth);
        }

        let scanned_count = Arc::new(AtomicU64::new(0));
        let mut skip_prefixes: Vec<PathBuf> = Vec::new();

        for entry in walker.into_iter().filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            // Skip hidden directories unless include_hidden (but allow .venv, .next, .nuxt, .gradle)
            if !self.options.include_hidden && name.starts_with('.') {
                // Allow certain dotfiles that are dev artifacts
                let allowed = [".venv", ".next", ".nuxt", ".gradle"];
                if !allowed.contains(&name.as_ref()) {
                    return false;
                }
            }
            true
        }) {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let name = entry.file_name().to_string_lossy();
            let path = entry.path();

            // Skip if this path is inside an already-found artifact directory
            if skip_prefixes.iter().any(|prefix| path.starts_with(prefix)) {
                continue;
            }

            if let Some(pb) = &progress {
                let count = scanned_count.fetch_add(1, Ordering::Relaxed);
                if count % 100 == 0 {
                    pb.set_message(format!("Scanned {} items...", count));
                }
            }

            for (pattern, file_type) in &patterns {
                if name == *pattern && entry.file_type().is_dir() {
                    let detected_type = FileType::detect(path);
                    let actual_type = if detected_type != FileType::Unknown {
                        detected_type
                    } else {
                        file_type.clone()
                    };

                    let (size, count) = calculate_dir_size(path);

                    if let Some(min_size) = self.options.min_size {
                        if size < min_size {
                            continue;
                        }
                    }

                    let last_modified = fs::metadata(path)
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64);

                    let scanned = ScannedEntry {
                        path: path.to_path_buf(),
                        file_type: actual_type.clone(),
                        size_bytes: size,
                        file_count: count,
                        last_modified,
                    };

                    by_type
                        .entry(actual_type.display_name().to_string())
                        .or_default()
                        .push(scanned.clone());

                    entries.push(scanned);

                    // Skip processing contents of this directory
                    skip_prefixes.push(path.to_path_buf());
                    break;
                }
            }
        }

        if let Some(pb) = progress {
            pb.finish_with_message("Scan complete");
        }

        let total_size: u64 = entries.iter().map(|e| e.size_bytes).sum();
        let total_files: u64 = entries.iter().map(|e| e.file_count).sum();

        Ok(ScanResult {
            root_path: root.to_path_buf(),
            entries,
            total_size,
            total_files,
            scan_duration_ms: start.elapsed().as_millis() as u64,
            by_type,
        })
    }

    /// Scan multiple directories in parallel
    pub fn scan_multiple(&self, roots: &[PathBuf]) -> Result<Vec<ScanResult>> {
        roots
            .par_iter()
            .map(|root| self.scan(root))
            .collect()
    }
}

/// Calculate the total size of a directory
pub fn calculate_dir_size(path: &Path) -> (u64, u64) {
    let mut total_size = 0u64;
    let mut file_count = 0u64;

    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            if let Ok(metadata) = entry.metadata() {
                total_size += metadata.len();
                file_count += 1;
            }
        }
    }

    (total_size, file_count)
}

/// Calculate the actual disk usage (not logical size) of a directory.
/// Uses blocks * 512 on Unix to correctly handle sparse files (e.g. Docker.raw).
pub fn calculate_dir_size_on_disk(path: &Path) -> (u64, u64) {
    let mut total_size = 0u64;
    let mut file_count = 0u64;

    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            if let Ok(metadata) = entry.metadata() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    // blocks are 512-byte blocks on Unix
                    total_size += metadata.blocks() * 512;
                }
                #[cfg(not(unix))]
                {
                    total_size += metadata.len();
                }
                file_count += 1;
            }
        }
    }

    (total_size, file_count)
}

/// Get the size of a single file or directory
pub fn get_size(path: &Path) -> u64 {
    if path.is_file() {
        fs::metadata(path).map(|m| m.len()).unwrap_or(0)
    } else if path.is_dir() {
        calculate_dir_size(path).0
    } else {
        0
    }
}

/// Find all directories matching a pattern
pub fn find_directories(root: &Path, pattern: &str) -> Vec<PathBuf> {
    let mut results = Vec::new();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
    {
        if entry.file_name().to_string_lossy() == pattern {
            results.push(entry.path().to_path_buf());
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_calculate_dir_size() {
        let temp_dir = env::temp_dir();
        let (size, _) = calculate_dir_size(&temp_dir);
        // Just check it doesn't panic
        assert!(size >= 0);
    }
}
