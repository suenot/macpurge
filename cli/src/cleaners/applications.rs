use anyhow::Result;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use super::{CleanOptions, CleanResult};
use crate::scanner::{calculate_dir_size, calculate_dir_size_on_disk};

/// Information about an installed application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub path: PathBuf,
    pub bundle_id: Option<String>,
    pub size_bytes: u64,
    pub data_size_bytes: u64,
    pub cache_size_bytes: u64,
    pub total_size_bytes: u64,
    pub version: Option<String>,
}

impl AppInfo {
    pub fn size_human(&self) -> String {
        crate::utils::format_size(self.size_bytes)
    }
}

/// Remnant files from uninstalled applications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRemnant {
    pub app_name: String,
    pub paths: Vec<PathBuf>,
    pub total_size: u64,
}

impl AppRemnant {
    pub fn size_human(&self) -> String {
        crate::utils::format_size(self.total_size)
    }
}

/// Scanner for installed applications
pub struct ApplicationScanner;

impl ApplicationScanner {
    /// Get list of all installed applications (fast — only app bundle size).
    /// Use `get_app_sizes()` to get data/cache sizes per-app on demand.
    pub fn list_apps() -> Result<Vec<AppInfo>> {
        let mut apps = Vec::new();
        let home = dirs::home_dir().unwrap_or_default();
        let app_dirs = vec![
            PathBuf::from("/Applications"),
            home.join("Applications"),
        ];

        for app_dir in app_dirs {
            if !app_dir.exists() {
                continue;
            }

            for entry in fs::read_dir(&app_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.extension().and_then(|e| e.to_str()) == Some("app") {
                    let name = path
                        .file_stem()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown")
                        .to_string();

                    let (size, _) = calculate_dir_size(&path);
                    let (bundle_id, version) = Self::get_app_info(&path);

                    apps.push(AppInfo {
                        name,
                        path,
                        bundle_id,
                        size_bytes: size,
                        data_size_bytes: 0,
                        cache_size_bytes: 0,
                        total_size_bytes: size,
                        version,
                    });
                }
            }
        }

        // Sort by size descending
        apps.sort_by(|a, b| b.total_size_bytes.cmp(&a.total_size_bytes));
        Ok(apps)
    }

    /// Get bundle ID and version from Info.plist
    fn get_app_info(app_path: &Path) -> (Option<String>, Option<String>) {
        let plist_path = app_path.join("Contents/Info.plist");

        if !plist_path.exists() {
            return (None, None);
        }

        // Use plutil to read plist
        let bundle_id = std::process::Command::new("defaults")
            .args(["read", &plist_path.to_string_lossy(), "CFBundleIdentifier"])
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                } else {
                    None
                }
            });

        let version = std::process::Command::new("defaults")
            .args([
                "read",
                &plist_path.to_string_lossy(),
                "CFBundleShortVersionString",
            ])
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                } else {
                    None
                }
            });

        (bundle_id, version)
    }

    /// Calculate data and cache sizes for a single app.
    /// Scans ~/Library subdirectories matching the app's name or bundle ID.
    /// This can be slow for apps with large data (e.g. Docker), so it's
    /// designed to be called per-app (not for all apps at once in list_apps).
    pub fn get_app_sizes(app_name: &str, bundle_id: Option<&str>) -> (u64, u64) {
        let home = dirs::home_dir().unwrap_or_default();
        let mut data_size = 0u64;
        let mut cache_size = 0u64;

        let data_dirs: &[&str] = &[
            "Library/Application Support",
            "Library/Containers",
            "Library/Saved Application State",
            "Library/Preferences",
            "Library/Group Containers",
        ];
        let cache_dirs: &[&str] = &[
            "Library/Caches",
            "Library/HTTPStorages",
            "Library/WebKit",
            "Library/Logs",
        ];

        let matches = |entry_name: &str| -> bool {
            if let Some(bid) = bundle_id {
                if entry_name == bid
                    || entry_name.starts_with(&format!("{}.", bid))
                    || entry_name.starts_with(&format!("group.{}", bid))
                {
                    return true;
                }
            }
            let lower = entry_name.to_lowercase();
            let name_lower = app_name.to_lowercase();
            lower == name_lower || lower.starts_with(&format!("{}.", name_lower))
        };

        for dir_name in data_dirs {
            let dir = home.join(dir_name);
            if !dir.exists() { continue; }
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if matches(&name) {
                        let p = entry.path();
                        // on-disk size: handles sparse files like Docker.raw, whose
                        // apparent length is the VM's max virtual disk (can be ~1 TB)
                        let size = if p.is_dir() {
                            calculate_dir_size_on_disk(&p).0
                        } else {
                            fs::metadata(&p).map(|m| m.len()).unwrap_or(0)
                        };
                        data_size += size;
                    }
                }
            }
        }

        for dir_name in cache_dirs {
            let dir = home.join(dir_name);
            if !dir.exists() { continue; }
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if matches(&name) {
                        let p = entry.path();
                        let size = if p.is_dir() {
                            calculate_dir_size_on_disk(&p).0
                        } else {
                            fs::metadata(&p).map(|m| m.len()).unwrap_or(0)
                        };
                        cache_size += size;
                    }
                }
            }
        }

        (data_size, cache_size)
    }

    /// Find remnants of uninstalled applications
    pub fn find_remnants() -> Result<Vec<AppRemnant>> {
        let home = dirs::home_dir().unwrap_or_default();

        // Get list of currently installed apps and their bundle IDs
        let installed_apps = Self::list_apps()?;
        let installed_names: HashSet<String> = installed_apps
            .iter()
            .map(|a| a.name.to_lowercase())
            .collect();
        let installed_bundle_ids: HashSet<String> = installed_apps
            .iter()
            .filter_map(|a| a.bundle_id.clone())
            .collect();

        // Directories to search for remnants
        let search_dirs = vec![
            home.join("Library/Application Support"),
            home.join("Library/Caches"),
            home.join("Library/Preferences"),
            home.join("Library/Saved Application State"),
            home.join("Library/Containers"),
            home.join("Library/Group Containers"),
        ];

        let mut remnants_map: std::collections::HashMap<String, Vec<PathBuf>> =
            std::collections::HashMap::new();

        for search_dir in search_dirs {
            if !search_dir.exists() {
                continue;
            }

            for entry in fs::read_dir(&search_dir)?.flatten() {
                let path = entry.path();
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                // Check if this looks like an app remnant
                let is_remnant = if name.contains('.') {
                    // Bundle ID style (com.example.app)
                    !installed_bundle_ids.contains(&name)
                        && name.split('.').count() >= 2
                        && !name.starts_with("com.apple")
                        && !name.starts_with("group.com.apple")
                } else {
                    // App name style
                    !installed_names.contains(&name.to_lowercase())
                        && !is_system_directory(&name)
                };

                if is_remnant {
                    remnants_map
                        .entry(name.clone())
                        .or_default()
                        .push(path);
                }
            }
        }

        // Also check LaunchAgents and LaunchDaemons
        let launch_dirs = vec![
            home.join("Library/LaunchAgents"),
            PathBuf::from("/Library/LaunchAgents"),
            PathBuf::from("/Library/LaunchDaemons"),
        ];

        for launch_dir in launch_dirs {
            if !launch_dir.exists() {
                continue;
            }

            for entry in fs::read_dir(&launch_dir)?.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("plist") {
                    let name = path
                        .file_stem()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();

                    if !installed_bundle_ids.contains(&name)
                        && !name.starts_with("com.apple")
                    {
                        remnants_map
                            .entry(name.clone())
                            .or_default()
                            .push(path);
                    }
                }
            }
        }

        // Convert to AppRemnant structs
        let mut remnants: Vec<AppRemnant> = remnants_map
            .into_iter()
            .map(|(name, paths)| {
                let total_size: u64 = paths
                    .iter()
                    .map(|p| {
                        if p.is_dir() {
                            calculate_dir_size(p).0
                        } else {
                            fs::metadata(p).map(|m| m.len()).unwrap_or(0)
                        }
                    })
                    .sum();

                AppRemnant {
                    app_name: name,
                    paths,
                    total_size,
                }
            })
            .filter(|r| r.total_size > 0) // Filter out empty remnants
            .collect();

        // Sort by size descending
        remnants.sort_by(|a, b| b.total_size.cmp(&a.total_size));
        Ok(remnants)
    }

    /// Find all files related to an application
    pub fn find_app_files(app_name: &str) -> Result<Vec<PathBuf>> {
        let home = dirs::home_dir().unwrap_or_default();
        let mut files = Vec::new();

        // Main application
        let app_paths = vec![
            PathBuf::from("/Applications").join(format!("{}.app", app_name)),
            home.join("Applications").join(format!("{}.app", app_name)),
        ];

        for path in app_paths {
            if path.exists() {
                files.push(path);
            }
        }

        // Support directories
        let search_dirs = vec![
            home.join("Library/Application Support"),
            home.join("Library/Caches"),
            home.join("Library/Preferences"),
            home.join("Library/Saved Application State"),
            home.join("Library/Containers"),
            home.join("Library/Logs"),
        ];

        let name_lower = app_name.to_lowercase();

        for search_dir in search_dirs {
            if !search_dir.exists() {
                continue;
            }

            for entry in WalkDir::new(&search_dir)
                .max_depth(2)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let entry_name = entry
                    .file_name()
                    .to_string_lossy()
                    .to_lowercase();

                if entry_name.contains(&name_lower) {
                    files.push(entry.path().to_path_buf());
                }
            }
        }

        Ok(files)
    }

    /// Uninstall an application completely
    pub fn uninstall_app(app_name: &str, options: &CleanOptions) -> Result<CleanResult> {
        let files = Self::find_app_files(app_name)?;
        let mut result = CleanResult::new();

        if files.is_empty() {
            result
                .errors
                .push(format!("No files found for application: {}", app_name));
            return Ok(result);
        }

        for path in files {
            let size = if path.is_dir() {
                calculate_dir_size(&path).0
            } else {
                fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
            };

            if options.verbose {
                let size_str = crate::utils::format_size(size);
                if options.dry_run {
                    println!("{} {} ({})", "[DRY RUN]".yellow(), path.display(), size_str);
                } else {
                    println!("{} {} ({})", "Removing".red(), path.display(), size_str);
                }
            }

            if !options.dry_run {
                let remove_result = if path.is_dir() {
                    fs::remove_dir_all(&path)
                } else {
                    fs::remove_file(&path)
                };

                match remove_result {
                    Ok(_) => {
                        result.items_removed += 1;
                        result.bytes_freed += size;
                        result.removed_paths.push(path);
                    }
                    Err(e) => {
                        result
                            .errors
                            .push(format!("Failed to remove {}: {}", path.display(), e));
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
}

/// Check if a directory name is a system directory that should not be considered a remnant
fn is_system_directory(name: &str) -> bool {
    let system_dirs = [
        "Apple",
        "CloudKit",
        "CoreData",
        "FileProvider",
        "GameKit",
        "HealthKit",
        "HomeKit",
        "Intents",
        "Maps",
        "Messages",
        "Metal",
        "MobileDevice",
        "Photos",
        "Safari",
        "SiriKit",
        "Speech",
        "StoreKit",
        "WebKit",
        "iCloud",
        "iTunes",
    ];

    system_dirs.iter().any(|d| name.eq_ignore_ascii_case(d))
}

/// Clean application remnants
pub fn clean_app_remnants(app_name: &str, options: &CleanOptions) -> Result<CleanResult> {
    let remnants = ApplicationScanner::find_remnants()?;
    let mut result = CleanResult::new();

    let name_lower = app_name.to_lowercase();

    for remnant in remnants {
        if remnant.app_name.to_lowercase().contains(&name_lower) {
            for path in remnant.paths {
                let size = if path.is_dir() {
                    calculate_dir_size(&path).0
                } else {
                    fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
                };

                if options.verbose {
                    let size_str = crate::utils::format_size(size);
                    if options.dry_run {
                        println!("{} {} ({})", "[DRY RUN]".yellow(), path.display(), size_str);
                    } else {
                        println!("{} {} ({})", "Removing".red(), path.display(), size_str);
                    }
                }

                if !options.dry_run {
                    let remove_result = if path.is_dir() {
                        fs::remove_dir_all(&path)
                    } else {
                        fs::remove_file(&path)
                    };

                    match remove_result {
                        Ok(_) => {
                            result.items_removed += 1;
                            result.bytes_freed += size;
                            result.removed_paths.push(path);
                        }
                        Err(e) => {
                            result.errors.push(format!(
                                "Failed to remove {}: {}",
                                path.display(),
                                e
                            ));
                        }
                    }
                } else {
                    result.items_removed += 1;
                    result.bytes_freed += size;
                    result.removed_paths.push(path);
                }
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_apps() {
        let apps = ApplicationScanner::list_apps();
        assert!(apps.is_ok());
    }

    #[test]
    fn test_is_system_directory() {
        assert!(is_system_directory("Apple"));
        assert!(is_system_directory("Safari"));
        assert!(!is_system_directory("SomeRandomApp"));
    }
}
