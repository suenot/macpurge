use anyhow::Result;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

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
        // Always scan the absolute /Applications; add ~/Applications only when a
        // usable absolute home is available, so a missing `$HOME` never yields a
        // relative path (GitLab #11).
        let mut app_dirs = vec![PathBuf::from("/Applications")];
        if let Some(home) = crate::utils::home_dir_checked() {
            app_dirs.push(home.join("Applications"));
        }

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
        // No usable absolute home => no home-derived directories to size, so we
        // report zero rather than scanning relative paths (GitLab #11).
        let home = match crate::utils::home_dir_checked() {
            Some(h) => h,
            None => return (0, 0),
        };
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
        // Every remnant search directory is home-derived; without a usable
        // absolute home there is nothing to scan and we return no remnants
        // rather than scanning relative paths (GitLab #11).
        let home = match crate::utils::home_dir_checked() {
            Some(h) => h,
            None => return Ok(Vec::new()),
        };

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

    /// Find all files related to an application.
    ///
    /// Matching is intentionally conservative to avoid the data-loss class of
    /// bug where a short/common name (e.g. "go", "code", "chat") matched
    /// unrelated apps' data via a loose substring test (GitLab #13). We prefer
    /// the app's bundle ID for an exact match and otherwise require a word/path
    /// boundary on the name. Generic or too-short names are rejected outright.
    pub fn find_app_files(app_name: &str) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        // Reject names that are too short or generic to match safely. Without a
        // boundary these would sweep up unrelated directories.
        if !is_safe_app_name(app_name) {
            anyhow::bail!(
                "Application name '{}' is too short or generic to uninstall safely. \
                 Please use the application's full name.",
                app_name
            );
        }

        // Every uninstall target below is home-derived; without a usable absolute
        // home we resolve no files rather than building relative delete paths that
        // could resolve under the CWD (GitLab #11).
        let home = match crate::utils::home_dir_checked() {
            Some(h) => h,
            None => return Ok(Vec::new()),
        };

        // Main application bundle. Resolve it first so we can read its bundle ID
        // and use that for an exact match against support/cache directories.
        let app_paths = vec![
            PathBuf::from("/Applications").join(format!("{}.app", app_name)),
            home.join("Applications").join(format!("{}.app", app_name)),
        ];

        let mut bundle_id: Option<String> = None;
        for path in app_paths {
            if path.exists() {
                if bundle_id.is_none() {
                    bundle_id = Self::get_app_info(&path).0;
                }
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

        for search_dir in search_dirs {
            if !search_dir.exists() {
                continue;
            }

            // Only inspect direct children of each support directory. The entry
            // name is the unit we match on (an app's data lives under a single
            // top-level dir/file named after the app or its bundle ID), so a
            // deeper walk only risks matching unrelated nested paths.
            if let Ok(entries) = fs::read_dir(&search_dir) {
                for entry in entries.flatten() {
                    let entry_name = entry.file_name().to_string_lossy().to_string();
                    if app_entry_matches(&entry_name, app_name, bundle_id.as_deref()) {
                        files.push(entry.path());
                    }
                }
            }
        }

        Ok(files)
    }

    /// Uninstall an application, deleting the exact list of paths previously
    /// found and confirmed.
    ///
    /// The caller passes the list produced by [`find_app_files`] so we delete
    /// precisely what was shown and confirmed — avoiding the TOCTOU window where
    /// a second scan at delete time could pick up different (or newly created)
    /// paths than the user approved.
    pub fn uninstall_app(files: &[PathBuf], options: &CleanOptions) -> Result<CleanResult> {
        let mut result = CleanResult::new();

        if files.is_empty() {
            result
                .errors
                .push("No files provided to uninstall".to_string());
            return Ok(result);
        }

        for path in files {
            let path = path.clone();
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
                match super::remove_path(&path, false, options.use_trash) {
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

/// Whether an app name is specific enough to match against data directories
/// without risking false positives. Names shorter than three characters or in a
/// short list of generic words are rejected, because without a boundary check
/// they would match unrelated directories (e.g. "go" inside "google-chrome").
fn is_safe_app_name(app_name: &str) -> bool {
    let trimmed = app_name.trim();
    if trimmed.chars().count() < 3 {
        return false;
    }

    // Obviously-generic names that, even with boundary matching, are too likely
    // to collide with shared/system directories or many apps at once.
    const GENERIC_NAMES: &[&str] = &[
        "app", "apps", "data", "cache", "caches", "log", "logs", "tmp", "temp",
        "support", "library", "user", "users", "shared", "common", "default",
    ];
    let lower = trimmed.to_lowercase();
    if GENERIC_NAMES.contains(&lower.as_str()) {
        return false;
    }

    true
}

/// Decide whether a support/cache directory entry belongs to the given app.
///
/// Matching rules (all case-insensitive), in order of preference:
/// 1. Bundle ID (when known): the entry equals the bundle ID exactly, or is a
///    child namespace of it (`<bundle_id>.`), or a group container
///    (`group.<bundle_id>` / `group.<bundle_id>.`). This is the most reliable
///    signal and is exact rather than substring.
/// 2. App name with a word/path boundary: the entry name with its extension
///    stripped equals the app name, or the entry begins with `<app_name>` *and*
///    the following character is a recognised separator (space, `-`, `_`, `.`).
///
/// We deliberately do NOT match when the app name is merely contained inside a
/// longer word (the GitLab #13 data-loss bug), so "go" never matches
/// "google-chrome" and "code" never matches "vscode-insiders".
fn app_entry_matches(entry_name: &str, app_name: &str, bundle_id: Option<&str>) -> bool {
    let entry_lower = entry_name.to_lowercase();

    // 1. Bundle ID (exact / namespaced / group container).
    if let Some(bid) = bundle_id {
        let bid_lower = bid.to_lowercase();
        if !bid_lower.is_empty()
            && (entry_lower == bid_lower
                || entry_lower.starts_with(&format!("{}.", bid_lower))
                || entry_lower == format!("group.{}", bid_lower)
                || entry_lower.starts_with(&format!("group.{}.", bid_lower)))
        {
            return true;
        }
    }

    // 2. App name with a boundary. Strip a single trailing extension so e.g.
    // "MyApp.plist" / "MyApp.savedState" match the bare app name.
    let name_lower = app_name.to_lowercase();
    let stem_lower = match entry_lower.rsplit_once('.') {
        Some((stem, _ext)) => stem,
        None => entry_lower.as_str(),
    };

    if stem_lower == name_lower {
        return true;
    }

    // Begins with the app name followed by a separator that marks a boundary, so
    // the name is a whole token rather than a prefix of a longer word.
    if let Some(rest) = entry_lower.strip_prefix(&name_lower) {
        if let Some(next) = rest.chars().next() {
            if matches!(next, ' ' | '-' | '_' | '.') {
                return true;
            }
        }
    }

    false
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

    // Reject names too short/generic to match safely (GitLab #13). Without this
    // a name like "go" would sweep up unrelated remnants.
    if !is_safe_app_name(app_name) {
        anyhow::bail!(
            "Application name '{}' is too short or generic to clean remnants safely.",
            app_name
        );
    }

    for remnant in remnants {
        // Use the same boundary-aware matcher as uninstall. A remnant's
        // `app_name` is the directory/bundle-id name; match it as a unit rather
        // than by loose substring containment.
        if app_entry_matches(&remnant.app_name, app_name, None) {
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
                    match super::remove_path(&path, false, options.use_trash) {
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

    /// Regression test for GitLab #13: a short app name must not match unrelated
    /// data directories via substring, but must still match its own exact-name
    /// directory.
    #[test]
    fn test_app_entry_matches_short_name_no_false_positive() {
        // "go" must NOT match unrelated apps' data dirs that merely contain it.
        assert!(!app_entry_matches("google-chrome", "go", None));
        assert!(!app_entry_matches("GoogleChrome", "Go", None));
        assert!(!app_entry_matches("org.golang.tools", "go", None));

        // But it MUST match the intended app's exact-name dir (boundary present).
        assert!(app_entry_matches("Go", "Go", None));
        assert!(app_entry_matches("go", "go", None));
        assert!(app_entry_matches("Go.plist", "Go", None));
        assert!(app_entry_matches("Go-helper", "Go", None));
    }

    #[test]
    fn test_app_entry_matches_boundary_required() {
        // Substring inside a longer word must not match.
        assert!(!app_entry_matches("vscode-insiders", "code", None));
        assert!(!app_entry_matches("notepad-plus", "note", None));

        // Whole-token / separator boundary matches are accepted.
        assert!(app_entry_matches("Code", "Code", None));
        assert!(app_entry_matches("Code.savedState", "Code", None));
        assert!(app_entry_matches("Note Helper", "Note", None));
    }

    #[test]
    fn test_app_entry_matches_bundle_id() {
        let bid = Some("com.example.myapp");
        // Exact bundle id and namespaced children match.
        assert!(app_entry_matches("com.example.myapp", "MyApp", bid));
        assert!(app_entry_matches("com.example.myapp.helper", "MyApp", bid));
        // Group containers match.
        assert!(app_entry_matches("group.com.example.myapp", "MyApp", bid));
        assert!(app_entry_matches("group.com.example.myapp.shared", "MyApp", bid));
        // A different bundle id sharing a prefix must NOT match.
        assert!(!app_entry_matches("com.example.myapptwo", "MyApp", bid));
        assert!(!app_entry_matches("com.example.other", "MyApp", bid));
    }

    #[test]
    fn test_is_safe_app_name() {
        // Too short.
        assert!(!is_safe_app_name("go"));
        assert!(!is_safe_app_name("a"));
        assert!(!is_safe_app_name(""));
        // Generic names rejected.
        assert!(!is_safe_app_name("cache"));
        assert!(!is_safe_app_name("App"));
        assert!(!is_safe_app_name("data"));
        // Specific names accepted.
        assert!(is_safe_app_name("Docker"));
        assert!(is_safe_app_name("Slack"));
        assert!(is_safe_app_name("Code"));
    }

    #[test]
    fn test_find_app_files_rejects_unsafe_name() {
        // A too-short name must return an error rather than a broad match.
        assert!(ApplicationScanner::find_app_files("go").is_err());
        assert!(ApplicationScanner::find_app_files("a").is_err());
    }
}
