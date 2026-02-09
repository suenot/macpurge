use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;
use std::process::Command;

use super::{CleanOptions, CleanResult, Cleaner};
use crate::scanner::calculate_dir_size;

/// Package manager cache cleaner
pub struct PackageManagerCleaner {
    manager: PackageManager,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PackageManager {
    Npm,
    Cargo,
    Homebrew,
    Pip,
    Gradle,
    Maven,
}

impl PackageManager {
    pub fn display_name(&self) -> &'static str {
        match self {
            PackageManager::Npm => "npm",
            PackageManager::Cargo => "Cargo",
            PackageManager::Homebrew => "Homebrew",
            PackageManager::Pip => "pip",
            PackageManager::Gradle => "Gradle",
            PackageManager::Maven => "Maven",
        }
    }

    pub fn cache_paths(&self) -> Vec<PathBuf> {
        let home = dirs::home_dir().unwrap_or_default();

        match self {
            PackageManager::Npm => vec![
                home.join(".npm/_cacache"),
                home.join(".npm/_logs"),
            ],
            PackageManager::Cargo => vec![
                home.join(".cargo/registry/cache"),
                home.join(".cargo/registry/src"),
                home.join(".cargo/git/db"),
                home.join(".cargo/git/checkouts"),
            ],
            PackageManager::Homebrew => vec![
                home.join("Library/Caches/Homebrew"),
                PathBuf::from("/usr/local/Homebrew/Library/Taps"),
            ],
            PackageManager::Pip => vec![
                home.join(".cache/pip"),
                home.join("Library/Caches/pip"),
            ],
            PackageManager::Gradle => vec![
                home.join(".gradle/caches"),
                home.join(".gradle/wrapper/dists"),
            ],
            PackageManager::Maven => vec![
                home.join(".m2/repository"),
            ],
        }
    }

    pub fn clean_command(&self) -> Option<(&'static str, Vec<&'static str>)> {
        match self {
            PackageManager::Npm => Some(("npm", vec!["cache", "clean", "--force"])),
            PackageManager::Cargo => None, // Manual cleanup needed
            PackageManager::Homebrew => Some(("brew", vec!["cleanup", "--prune=all"])),
            PackageManager::Pip => Some(("pip", vec!["cache", "purge"])),
            PackageManager::Gradle => None, // Manual cleanup needed
            PackageManager::Maven => None, // Manual cleanup needed
        }
    }

    pub fn is_installed(&self) -> bool {
        let cmd = match self {
            PackageManager::Npm => "npm",
            PackageManager::Cargo => "cargo",
            PackageManager::Homebrew => "brew",
            PackageManager::Pip => "pip3",
            PackageManager::Gradle => "gradle",
            PackageManager::Maven => "mvn",
        };

        Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

impl PackageManagerCleaner {
    pub fn new(manager: PackageManager) -> Self {
        Self { manager }
    }

    pub fn npm() -> Self {
        Self::new(PackageManager::Npm)
    }

    pub fn cargo() -> Self {
        Self::new(PackageManager::Cargo)
    }

    pub fn homebrew() -> Self {
        Self::new(PackageManager::Homebrew)
    }

    pub fn pip() -> Self {
        Self::new(PackageManager::Pip)
    }

    pub fn gradle() -> Self {
        Self::new(PackageManager::Gradle)
    }

    pub fn maven() -> Self {
        Self::new(PackageManager::Maven)
    }

    fn get_cache_size(&self) -> u64 {
        self.manager
            .cache_paths()
            .iter()
            .filter(|p| p.exists())
            .map(|p| calculate_dir_size(p).0)
            .sum()
    }
}

impl Cleaner for PackageManagerCleaner {
    fn name(&self) -> &'static str {
        match self.manager {
            PackageManager::Npm => "npm Cache Cleaner",
            PackageManager::Cargo => "Cargo Cache Cleaner",
            PackageManager::Homebrew => "Homebrew Cache Cleaner",
            PackageManager::Pip => "pip Cache Cleaner",
            PackageManager::Gradle => "Gradle Cache Cleaner",
            PackageManager::Maven => "Maven Cache Cleaner",
        }
    }

    fn description(&self) -> &'static str {
        match self.manager {
            PackageManager::Npm => "Clean npm global cache",
            PackageManager::Cargo => "Clean Cargo registry cache",
            PackageManager::Homebrew => "Clean Homebrew cache and old versions",
            PackageManager::Pip => "Clean pip download cache",
            PackageManager::Gradle => "Clean Gradle caches",
            PackageManager::Maven => "Clean Maven local repository",
        }
    }

    fn clean(&self, options: &CleanOptions) -> Result<CleanResult> {
        let mut result = CleanResult::new();

        if !self.manager.is_installed() {
            result.errors.push(format!(
                "{} is not installed",
                self.manager.display_name()
            ));
            return Ok(result);
        }

        let initial_size = self.get_cache_size();

        // Try using the native clean command if available
        if let Some((cmd, args)) = self.manager.clean_command() {
            if options.verbose {
                println!(
                    "{} {} {}",
                    "Running".cyan(),
                    cmd,
                    args.join(" ")
                );
            }

            if !options.dry_run {
                let output = Command::new(cmd).args(&args).output();

                match output {
                    Ok(o) if o.status.success() => {
                        let final_size = self.get_cache_size();
                        result.bytes_freed = initial_size.saturating_sub(final_size);
                        result.items_removed = 1;
                    }
                    Ok(o) => {
                        result.errors.push(format!(
                            "Command failed: {}",
                            String::from_utf8_lossy(&o.stderr)
                        ));
                    }
                    Err(e) => {
                        result.errors.push(format!("Failed to run command: {}", e));
                    }
                }
            } else {
                result.bytes_freed = initial_size;
                result.items_removed = 1;
            }
        } else {
            // Manual cleanup for managers without clean command
            for cache_path in self.manager.cache_paths() {
                if !cache_path.exists() {
                    continue;
                }

                let (size, _) = calculate_dir_size(&cache_path);

                if options.verbose {
                    let size_str = crate::utils::format_size(size);
                    if options.dry_run {
                        println!(
                            "{} {} ({})",
                            "[DRY RUN]".yellow(),
                            cache_path.display(),
                            size_str
                        );
                    } else {
                        println!(
                            "{} {} ({})",
                            "Removing".red(),
                            cache_path.display(),
                            size_str
                        );
                    }
                }

                if !options.dry_run {
                    match std::fs::remove_dir_all(&cache_path) {
                        Ok(_) => {
                            result.items_removed += 1;
                            result.bytes_freed += size;
                            result.removed_paths.push(cache_path);
                        }
                        Err(e) => {
                            result.errors.push(format!(
                                "Failed to remove {}: {}",
                                cache_path.display(),
                                e
                            ));
                        }
                    }
                } else {
                    result.items_removed += 1;
                    result.bytes_freed += size;
                    result.removed_paths.push(cache_path);
                }
            }
        }

        Ok(result)
    }

    fn estimate(&self) -> Result<u64> {
        Ok(self.get_cache_size())
    }
}

/// Get information about all package manager caches
pub fn list_package_manager_caches() -> Vec<(PackageManager, u64, bool)> {
    let managers = [
        PackageManager::Npm,
        PackageManager::Cargo,
        PackageManager::Homebrew,
        PackageManager::Pip,
        PackageManager::Gradle,
        PackageManager::Maven,
    ];

    managers
        .into_iter()
        .map(|m| {
            let cleaner = PackageManagerCleaner::new(m);
            let size = cleaner.get_cache_size();
            let installed = m.is_installed();
            (m, size, installed)
        })
        .collect()
}

/// Clean npm cache
pub fn clean_npm_cache(options: &CleanOptions) -> Result<CleanResult> {
    PackageManagerCleaner::npm().clean(options)
}

/// Clean cargo cache
pub fn clean_cargo_cache(options: &CleanOptions) -> Result<CleanResult> {
    PackageManagerCleaner::cargo().clean(options)
}

/// Clean homebrew cache
pub fn clean_homebrew_cache(options: &CleanOptions) -> Result<CleanResult> {
    PackageManagerCleaner::homebrew().clean(options)
}

/// Clean pip cache
pub fn clean_pip_cache(options: &CleanOptions) -> Result<CleanResult> {
    PackageManagerCleaner::pip().clean(options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_manager_cleaner_creation() {
        let cleaner = PackageManagerCleaner::npm();
        assert_eq!(cleaner.name(), "npm Cache Cleaner");
    }

    #[test]
    fn test_cache_paths() {
        let paths = PackageManager::Npm.cache_paths();
        assert!(!paths.is_empty());
    }
}
