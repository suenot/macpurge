use anyhow::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};

use super::{CleanOptions, CleanResult, Cleaner};
use crate::scanner::{DirectoryScanner, FileType, ScanOptions, ScannedEntry};

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
        let scanner = DirectoryScanner::new(ScanOptions {
            dev_mode: true,
            show_progress: false,
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
        let entries = self.get_entries()?;
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

    #[test]
    fn test_dev_artifacts_cleaner_creation() {
        let cleaner = DevArtifactsCleaner::node_modules(PathBuf::from("/tmp"));
        assert_eq!(cleaner.name(), "Node.js Modules Cleaner");
    }
}
