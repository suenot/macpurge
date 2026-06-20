//! AI / neural-network model cleaner.
//!
//! Removes model artifacts discovered by [`crate::scanner::AiModelScanner`].
//! Each artifact is identified by an absolute path (a model directory, a
//! Hugging Face repo, the Xet store, or the Ollama model store).

use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;

use super::{CleanOptions, CleanResult, Cleaner};
use crate::scanner::calculate_dir_size;

/// Cleaner for a single AI model artifact path.
pub struct AiModelCleaner {
    path: PathBuf,
}

impl AiModelCleaner {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Cleaner for AiModelCleaner {
    fn name(&self) -> &'static str {
        "AI Model Cleaner"
    }

    fn description(&self) -> &'static str {
        "Remove a downloaded AI/ML model artifact"
    }

    fn clean(&self, options: &CleanOptions) -> Result<CleanResult> {
        let mut result = CleanResult::new();

        if !self.path.exists() {
            return Ok(result);
        }

        let size = if self.path.is_dir() {
            let (s, _) = calculate_dir_size(&self.path);
            s
        } else {
            self.path.metadata().map(|m| m.len()).unwrap_or(0)
        };

        if options.verbose {
            let size_str = crate::utils::format_size(size);
            if options.dry_run {
                println!("{} {} ({})", "[DRY RUN]".yellow(), self.path.display(), size_str);
            } else {
                println!("{} {} ({})", "Removing".red(), self.path.display(), size_str);
            }
        }

        if options.dry_run {
            result.items_removed += 1;
            result.bytes_freed += size;
            result.removed_paths.push(self.path.clone());
            return Ok(result);
        }

        let removal = if self.path.is_dir() {
            std::fs::remove_dir_all(&self.path)
        } else {
            std::fs::remove_file(&self.path)
        };

        match removal {
            Ok(_) => {
                result.items_removed += 1;
                result.bytes_freed += size;
                result.removed_paths.push(self.path.clone());
            }
            Err(e) => {
                result
                    .errors
                    .push(format!("Failed to remove {}: {}", self.path.display(), e));
            }
        }

        Ok(result)
    }

    fn estimate(&self) -> Result<u64> {
        if !self.path.exists() {
            return Ok(0);
        }
        if self.path.is_dir() {
            let (size, _) = calculate_dir_size(&self.path);
            Ok(size)
        } else {
            Ok(self.path.metadata().map(|m| m.len()).unwrap_or(0))
        }
    }
}

/// Clean a single AI model artifact path.
pub fn clean_ai_model(path: PathBuf, options: &CleanOptions) -> Result<CleanResult> {
    AiModelCleaner::new(path).clean(options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleaner_name() {
        let cleaner = AiModelCleaner::new(PathBuf::from("/nonexistent/path"));
        assert_eq!(cleaner.name(), "AI Model Cleaner");
    }

    #[test]
    fn test_clean_missing_path_is_noop() {
        let cleaner = AiModelCleaner::new(PathBuf::from("/nonexistent/path/xyz"));
        let result = cleaner.clean(&CleanOptions::default()).unwrap();
        assert_eq!(result.items_removed, 0);
        assert_eq!(result.bytes_freed, 0);
    }
}
