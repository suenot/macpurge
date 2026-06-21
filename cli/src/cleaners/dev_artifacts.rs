use anyhow::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::{CleanOptions, CleanResult, Cleaner};
use crate::scanner::{DirectoryScanner, FileType, ScanOptions, ScannedEntry};

/// Number of seconds in a day, used to translate `min_age_days` into a cutoff.
const SECONDS_PER_DAY: u64 = 86_400;

/// Decide whether an artifact entry is old enough to be deleted under the
/// configured `min_age_days` filter.
///
/// We rely on the directory's own modification time, which the scanner captures
/// into `ScannedEntry::last_modified` (seconds since the UNIX epoch) from
/// `metadata.modified()` at scan time. Using the directory's own mtime is both
/// correct and simple here: cleaning tools (cargo, npm, pip) rewrite the
/// artifact directory itself whenever they touch its contents, so the dir mtime
/// reliably tracks "last time this artifact was used".
///
/// Behavior:
/// - `None`  -> no age filter; every entry is eligible (unchanged behavior).
/// - `Some(days)` -> an entry is eligible only if its modification time is at or
///   before `now - days`. Entries modified more recently than the cutoff (i.e.
///   actively-used projects) are skipped.
///
/// We are deliberately CONSERVATIVE: if the entry has no recorded modification
/// time (the scanner could not read it), we skip deletion rather than risk
/// removing something we cannot age-check while a filter is requested.
fn is_old_enough(entry: &ScannedEntry, min_age_days: Option<u64>) -> bool {
    let days = match min_age_days {
        None => return true, // no filter requested
        Some(days) => days,
    };

    // Cutoff in epoch seconds. `now - days*86400`; saturate at 0 so an absurdly
    // large `days` simply makes everything "too new" rather than overflowing.
    let now_secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let cutoff = now_secs.saturating_sub(days.saturating_mul(SECONDS_PER_DAY));

    match entry.last_modified {
        // Eligible only if it was last modified at or before the cutoff.
        Some(modified_secs) => (modified_secs.max(0) as u64) <= cutoff,
        // Conservative: we couldn't age-check this entry, so don't delete it.
        None => false,
    }
}

/// Cleaner for development artifacts (node_modules, venv, target, etc.)
pub struct DevArtifactsCleaner {
    root: PathBuf,
    artifact_type: DevArtifactType,
}

#[derive(Debug, Clone, Copy)]
pub enum DevArtifactType {
    NodeModules,
    PythonVenv,
    RustTarget,
    All,
}

impl DevArtifactsCleaner {
    pub fn new(root: PathBuf, artifact_type: DevArtifactType) -> Self {
        Self { root, artifact_type }
    }

    pub fn node_modules(root: PathBuf) -> Self {
        Self::new(root, DevArtifactType::NodeModules)
    }

    pub fn python_venv(root: PathBuf) -> Self {
        Self::new(root, DevArtifactType::PythonVenv)
    }

    pub fn rust_target(root: PathBuf) -> Self {
        Self::new(root, DevArtifactType::RustTarget)
    }

    pub fn all(root: PathBuf) -> Self {
        Self::new(root, DevArtifactType::All)
    }

    fn get_entries(&self) -> Result<Vec<ScannedEntry>> {
        // Honor the global "never scan" exclude list (loaded once per scan).
        let excluded = crate::config::get_excluded_dirs(
            &crate::config::load_config().unwrap_or_default(),
        );
        let scanner = DirectoryScanner::new(ScanOptions {
            dev_mode: true,
            show_progress: false,
            excluded,
            ..Default::default()
        });

        let result = scanner.scan(&self.root)?;

        let filtered: Vec<ScannedEntry> = match self.artifact_type {
            DevArtifactType::NodeModules => result
                .entries
                .into_iter()
                .filter(|e| e.file_type == FileType::NodeModules)
                .collect(),
            DevArtifactType::PythonVenv => result
                .entries
                .into_iter()
                .filter(|e| matches!(e.file_type, FileType::PythonVenv | FileType::PythonCache))
                .collect(),
            DevArtifactType::RustTarget => result
                .entries
                .into_iter()
                .filter(|e| e.file_type == FileType::RustTarget)
                .collect(),
            DevArtifactType::All => result.entries,
        };

        Ok(filtered)
    }
}

impl Cleaner for DevArtifactsCleaner {
    fn name(&self) -> &'static str {
        match self.artifact_type {
            DevArtifactType::NodeModules => "Node.js Modules Cleaner",
            DevArtifactType::PythonVenv => "Python Venv Cleaner",
            DevArtifactType::RustTarget => "Rust Target Cleaner",
            DevArtifactType::All => "Dev Artifacts Cleaner",
        }
    }

    fn description(&self) -> &'static str {
        match self.artifact_type {
            DevArtifactType::NodeModules => "Remove node_modules directories",
            DevArtifactType::PythonVenv => "Remove Python virtual environments",
            DevArtifactType::RustTarget => "Remove Rust target directories",
            DevArtifactType::All => "Remove all development artifacts",
        }
    }

    fn clean(&self, options: &CleanOptions) -> Result<CleanResult> {
        // Honor the `min_age_days` filter: when set, only delete artifacts that
        // have NOT been modified within the cutoff window. This protects
        // actively-used projects (e.g. node_modules/target/venv in a project
        // worked on today) from being purged by the unattended scheduler.
        let entries: Vec<ScannedEntry> = self
            .get_entries()?
            .into_iter()
            .filter(|e| is_old_enough(e, options.min_age_days))
            .collect();
        let mut result = CleanResult::new();

        if entries.is_empty() {
            return Ok(result);
        }

        let pb = ProgressBar::new(entries.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );

        for entry in entries {
            pb.set_message(format!("Cleaning {}", entry.path.display()));

            if options.verbose {
                if options.dry_run {
                    println!(
                        "{} {} ({})",
                        "[DRY RUN]".yellow(),
                        entry.path.display(),
                        entry.size_human()
                    );
                } else {
                    println!(
                        "{} {} ({})",
                        "Removing".red(),
                        entry.path.display(),
                        entry.size_human()
                    );
                }
            }

            if !options.dry_run {
                match super::remove_path(&entry.path, false, options.use_trash) {
                    Ok(_) => {
                        result.items_removed += 1;
                        result.bytes_freed += entry.size_bytes;
                        result.removed_paths.push(entry.path);
                    }
                    Err(e) => {
                        result
                            .errors
                            .push(format!("Failed to remove {}: {}", entry.path.display(), e));
                    }
                }
            } else {
                result.items_removed += 1;
                result.bytes_freed += entry.size_bytes;
                result.removed_paths.push(entry.path);
            }

            pb.inc(1);
        }

        pb.finish_with_message("Done");
        Ok(result)
    }

    fn estimate(&self) -> Result<u64> {
        let entries = self.get_entries()?;
        Ok(entries.iter().map(|e| e.size_bytes).sum())
    }
}

/// Clean node_modules in a directory
pub fn clean_node_modules(root: &Path, options: &CleanOptions) -> Result<CleanResult> {
    let cleaner = DevArtifactsCleaner::node_modules(root.to_path_buf());
    cleaner.clean(options)
}

/// Clean Python virtual environments in a directory
pub fn clean_python_venv(root: &Path, options: &CleanOptions) -> Result<CleanResult> {
    let cleaner = DevArtifactsCleaner::python_venv(root.to_path_buf());
    cleaner.clean(options)
}

/// Clean Rust target directories
pub fn clean_rust_target(root: &Path, options: &CleanOptions) -> Result<CleanResult> {
    let cleaner = DevArtifactsCleaner::rust_target(root.to_path_buf());
    cleaner.clean(options)
}

/// Clean all dev artifacts
pub fn clean_all_dev_artifacts(root: &Path, options: &CleanOptions) -> Result<CleanResult> {
    let cleaner = DevArtifactsCleaner::all(root.to_path_buf());
    cleaner.clean(options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_dev_artifacts_cleaner_creation() {
        let cleaner = DevArtifactsCleaner::node_modules(PathBuf::from("/tmp"));
        assert_eq!(cleaner.name(), "Node.js Modules Cleaner");
    }

    /// Helper to build a `ScannedEntry` with a chosen modification time.
    fn entry_modified_secs_ago(secs_ago: u64) -> ScannedEntry {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        ScannedEntry {
            path: PathBuf::from("/tmp/some-project/node_modules"),
            file_type: FileType::NodeModules,
            size_bytes: 1024,
            file_count: 1,
            last_modified: Some((now - secs_ago) as i64),
        }
    }

    #[test]
    fn test_min_age_days_filter() {
        // No filter: always eligible, regardless of age (unchanged behavior).
        let recent = entry_modified_secs_ago(60); // modified a minute ago
        assert!(is_old_enough(&recent, None));

        // With a 30-day filter, a directory modified a minute ago is too new
        // and must NOT be deleted (protects actively-used projects).
        assert!(!is_old_enough(&recent, Some(30)));

        // With a 30-day filter, a directory modified 60 days ago IS old enough.
        let old = entry_modified_secs_ago(60 * SECONDS_PER_DAY);
        assert!(is_old_enough(&old, Some(30)));

        // Conservative: an entry whose mtime could not be read is skipped when a
        // filter is requested, but still eligible when no filter is set.
        let unknown = ScannedEntry {
            path: PathBuf::from("/tmp/unknown/node_modules"),
            file_type: FileType::NodeModules,
            size_bytes: 1024,
            file_count: 1,
            last_modified: None,
        };
        assert!(!is_old_enough(&unknown, Some(30)));
        assert!(is_old_enough(&unknown, None));
    }

    #[test]
    fn test_clean_respects_min_age_days_for_recent_dir() {
        // Create a real (freshly-modified) artifact directory on disk. Because it
        // was just created, its mtime is "now", so a 30-day age filter must spare
        // it while no filter (None) would remove it.
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("my-app");
        let node_modules = project.join("node_modules");
        fs::create_dir_all(&node_modules).unwrap();
        fs::write(node_modules.join("dummy.txt"), b"x").unwrap();

        let cleaner = DevArtifactsCleaner::node_modules(project.clone());

        // With a 30-day minimum age, the just-created dir is too new: nothing
        // should be removed and it must still exist.
        let opts_filtered = CleanOptions {
            dry_run: false,
            force: false,
            verbose: false,
            min_age_days: Some(30),
            use_trash: false,
        };
        let result = cleaner.clean(&opts_filtered).unwrap();
        assert_eq!(
            result.items_removed, 0,
            "recent node_modules should be skipped under a 30-day filter"
        );
        assert!(node_modules.exists(), "recent dir must not be deleted");

        // With no age filter, the same directory IS removed (unchanged behavior).
        let opts_unfiltered = CleanOptions {
            dry_run: false,
            force: false,
            verbose: false,
            min_age_days: None,
            use_trash: false,
        };
        let result = cleaner.clean(&opts_unfiltered).unwrap();
        assert_eq!(
            result.items_removed, 1,
            "without a filter the dir should be removed"
        );
        assert!(!node_modules.exists(), "dir should be deleted when no filter set");
    }
}
