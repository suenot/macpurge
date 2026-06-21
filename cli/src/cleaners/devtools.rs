//! Dev tools cleaner for Xcode, Android/Gradle, Docker caches

use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;
use std::process::Command;

use super::{CleanOptions, CleanResult, Cleaner};
use crate::scanner::calculate_dir_size;

/// Types of dev tool caches that can be cleaned
#[derive(Debug, Clone)]
pub enum DevToolCleanType {
    XcodeDerivedData,
    XcodeArchives,
    XcodeSimulators,
    XcodeDeviceSupport,
    XcodeCache,
    GradleCache,
    DockerDangling,
    DockerAll,
    /// Clean a specific path
    Path(PathBuf),
}

/// Cleaner for development tool caches
pub struct DevToolsCleaner {
    clean_type: DevToolCleanType,
}

impl DevToolsCleaner {
    pub fn new(clean_type: DevToolCleanType) -> Self {
        Self { clean_type }
    }

    pub fn xcode_derived_data() -> Self {
        Self::new(DevToolCleanType::XcodeDerivedData)
    }

    pub fn xcode_archives() -> Self {
        Self::new(DevToolCleanType::XcodeArchives)
    }

    pub fn xcode_simulators() -> Self {
        Self::new(DevToolCleanType::XcodeSimulators)
    }

    pub fn xcode_device_support() -> Self {
        Self::new(DevToolCleanType::XcodeDeviceSupport)
    }

    pub fn xcode_cache() -> Self {
        Self::new(DevToolCleanType::XcodeCache)
    }

    pub fn gradle_cache() -> Self {
        Self::new(DevToolCleanType::GradleCache)
    }

    pub fn docker_dangling() -> Self {
        Self::new(DevToolCleanType::DockerDangling)
    }

    pub fn docker_all() -> Self {
        Self::new(DevToolCleanType::DockerAll)
    }

    pub fn path(path: PathBuf) -> Self {
        Self::new(DevToolCleanType::Path(path))
    }

    fn get_paths(&self) -> Vec<PathBuf> {
        let home = dirs::home_dir().unwrap_or_default();
        let mut paths = Vec::new();

        match &self.clean_type {
            DevToolCleanType::XcodeDerivedData => {
                paths.push(home.join("Library/Developer/Xcode/DerivedData"));
            }
            DevToolCleanType::XcodeArchives => {
                paths.push(home.join("Library/Developer/Xcode/Archives"));
            }
            DevToolCleanType::XcodeSimulators => {
                paths.push(home.join("Library/Developer/CoreSimulator/Devices"));
                paths.push(home.join("Library/Developer/CoreSimulator/Caches"));
            }
            DevToolCleanType::XcodeDeviceSupport => {
                paths.push(home.join("Library/Developer/Xcode/iOS DeviceSupport"));
                paths.push(home.join("Library/Developer/Xcode/watchOS DeviceSupport"));
            }
            DevToolCleanType::XcodeCache => {
                paths.push(home.join("Library/Caches/com.apple.dt.Xcode"));
            }
            DevToolCleanType::GradleCache => {
                paths.push(home.join(".gradle/caches"));
                paths.push(home.join(".gradle/wrapper/dists"));
            }
            DevToolCleanType::DockerDangling | DevToolCleanType::DockerAll => {
                // Docker cleaning is handled via CLI commands, not path removal
            }
            DevToolCleanType::Path(p) => {
                paths.push(p.clone());
            }
        }

        paths.into_iter().filter(|p| p.exists()).collect()
    }
}

impl Cleaner for DevToolsCleaner {
    fn name(&self) -> &'static str {
        match &self.clean_type {
            DevToolCleanType::XcodeDerivedData => "Xcode DerivedData Cleaner",
            DevToolCleanType::XcodeArchives => "Xcode Archives Cleaner",
            DevToolCleanType::XcodeSimulators => "Xcode Simulators Cleaner",
            DevToolCleanType::XcodeDeviceSupport => "Xcode DeviceSupport Cleaner",
            DevToolCleanType::XcodeCache => "Xcode Cache Cleaner",
            DevToolCleanType::GradleCache => "Gradle Cache Cleaner",
            DevToolCleanType::DockerDangling => "Docker Dangling Cleaner",
            DevToolCleanType::DockerAll => "Docker Full Cleaner",
            DevToolCleanType::Path(_) => "Path Cleaner",
        }
    }

    fn description(&self) -> &'static str {
        match &self.clean_type {
            DevToolCleanType::XcodeDerivedData => "Remove Xcode build artifacts",
            DevToolCleanType::XcodeArchives => "Remove old Xcode archives",
            DevToolCleanType::XcodeSimulators => "Remove simulator data and caches",
            DevToolCleanType::XcodeDeviceSupport => "Remove device debug symbols",
            DevToolCleanType::XcodeCache => "Remove Xcode caches",
            DevToolCleanType::GradleCache => "Remove Gradle build caches",
            DevToolCleanType::DockerDangling => "Remove dangling Docker images and stopped containers",
            DevToolCleanType::DockerAll => "Full Docker system cleanup",
            DevToolCleanType::Path(_) => "Remove a specific dev tool cache path",
        }
    }

    fn clean(&self, options: &CleanOptions) -> Result<CleanResult> {
        // Docker cleaning uses CLI commands
        match &self.clean_type {
            DevToolCleanType::DockerDangling => return clean_docker_cmd(options, false),
            DevToolCleanType::DockerAll => return clean_docker_cmd(options, true),
            _ => {}
        }

        let paths = self.get_paths();
        let mut result = CleanResult::new();

        for path in paths {
            let (size, _) = calculate_dir_size(&path);

            if options.verbose {
                let size_str = crate::utils::format_size(size);
                if options.dry_run {
                    println!("{} {} ({})", "[DRY RUN]".yellow(), path.display(), size_str);
                } else {
                    println!("{} {} ({})", "Removing".red(), path.display(), size_str);
                }
            }

            if !options.dry_run {
                match super::remove_path(&path, false, options.use_trash) {
                    Ok(_) => {
                        result.items_removed += 1;
                        result.bytes_freed += size;
                        result.removed_paths.push(path);
                    }
                    Err(e) => {
                        result.errors.push(format!("Failed to remove {}: {}", path.display(), e));
                    }
                }
            } else {
                result.items_removed += 1;
                result.bytes_freed += size;
                result.removed_paths.push(path);
            }
        }

        Ok(result)
    }

    fn estimate(&self) -> Result<u64> {
        match &self.clean_type {
            DevToolCleanType::DockerDangling | DevToolCleanType::DockerAll => {
                // Can't easily estimate Docker without running docker system df
                Ok(0)
            }
            _ => {
                let paths = self.get_paths();
                let total: u64 = paths.iter().map(|p| {
                    let (size, _) = calculate_dir_size(p);
                    size
                }).sum();
                Ok(total)
            }
        }
    }
}

/// Run Docker system prune via CLI
fn clean_docker_cmd(options: &CleanOptions, all: bool) -> Result<CleanResult> {
    let mut result = CleanResult::new();

    if !which::which("docker").is_ok() {
        result.errors.push("Docker is not installed".to_string());
        return Ok(result);
    }

    if options.dry_run {
        if options.verbose {
            println!("{} docker system prune {}", "[DRY RUN]".yellow(), if all { "--all" } else { "" });
        }
        return Ok(result);
    }

    let mut args = vec!["system", "prune", "-f"];
    if all {
        args.push("--all");
        args.push("--volumes");
    }

    match Command::new("docker").args(&args).output() {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Try to parse reclaimed space from output
                // Docker outputs "Total reclaimed space: X.YGB"
                for line in stdout.lines() {
                    if line.contains("reclaimed space") {
                        if let Some(size_part) = line.split(':').nth(1) {
                            let size = crate::scanner::devtools::parse_docker_size_pub(size_part.trim());
                            result.bytes_freed += size;
                        }
                    }
                }
                result.items_removed += 1;
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                result.errors.push(format!("Docker prune failed: {}", stderr));
            }
        }
        Err(e) => {
            result.errors.push(format!("Failed to run docker: {}", e));
        }
    }

    Ok(result)
}

// ── Convenience functions ─────────────────────────────────────

/// Clean Xcode DerivedData
pub fn clean_xcode_derived_data(options: &CleanOptions) -> Result<CleanResult> {
    DevToolsCleaner::xcode_derived_data().clean(options)
}

/// Clean Docker dangling images/containers
pub fn clean_docker_dangling(options: &CleanOptions) -> Result<CleanResult> {
    DevToolsCleaner::docker_dangling().clean(options)
}

/// Clean Gradle cache
pub fn clean_gradle_cache(options: &CleanOptions) -> Result<CleanResult> {
    DevToolsCleaner::gradle_cache().clean(options)
}

/// Clean a specific dev tool cache path
pub fn clean_devtool_path(path: PathBuf, options: &CleanOptions) -> Result<CleanResult> {
    DevToolsCleaner::path(path).clean(options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_devtools_cleaner_creation() {
        let cleaner = DevToolsCleaner::xcode_derived_data();
        assert_eq!(cleaner.name(), "Xcode DerivedData Cleaner");
    }

    #[test]
    fn test_all_cleaner_types() {
        assert_eq!(DevToolsCleaner::gradle_cache().name(), "Gradle Cache Cleaner");
        assert_eq!(DevToolsCleaner::docker_dangling().name(), "Docker Dangling Cleaner");
        assert_eq!(DevToolsCleaner::xcode_simulators().name(), "Xcode Simulators Cleaner");
    }
}
