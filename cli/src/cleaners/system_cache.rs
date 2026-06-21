use anyhow::Result;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;
use walkdir::WalkDir;

use super::{CleanOptions, CleanResult, Cleaner};
use crate::scanner::calculate_dir_size;

/// System cache locations on macOS
pub struct SystemCacheCleaner {
    cache_type: SystemCacheType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemCacheType {
    UserCache,
    SystemCache,
    BrowserCache,
    SystemLogs,
    IOSBackups,
    Trash,
    DownloadsOld,
    MailCaches,
    All,
}

impl SystemCacheType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::UserCache => "User Cache",
            Self::SystemCache => "System Cache",
            Self::BrowserCache => "Browser Cache",
            Self::SystemLogs => "System Logs",
            Self::IOSBackups => "iOS Backups",
            Self::Trash => "Trash",
            Self::DownloadsOld => "Old Downloads",
            Self::MailCaches => "Mail Caches",
            Self::All => "All",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::UserCache => "Application cache files in ~/Library/Caches",
            Self::SystemCache => "System-level cache files",
            Self::BrowserCache => "Browser cache files (Chrome, Safari, Firefox, etc.)",
            Self::SystemLogs => "System and application log files",
            Self::IOSBackups => "iOS device backup files",
            Self::Trash => "Files in the Trash",
            Self::DownloadsOld => "Old files in ~/Downloads",
            Self::MailCaches => "Mail application cache files (does not touch the ~/Library/Mail message store)",
            Self::All => "All system caches and cleanable data",
        }
    }

    pub fn all_categories() -> Vec<Self> {
        vec![
            Self::UserCache,
            Self::BrowserCache,
            Self::SystemLogs,
            Self::IOSBackups,
            Self::Trash,
            Self::DownloadsOld,
            Self::MailCaches,
        ]
    }
}

impl SystemCacheCleaner {
    pub fn new(cache_type: SystemCacheType) -> Self {
        Self { cache_type }
    }

    pub fn user_cache() -> Self {
        Self::new(SystemCacheType::UserCache)
    }

    pub fn system_cache() -> Self {
        Self::new(SystemCacheType::SystemCache)
    }

    pub fn browser_cache() -> Self {
        Self::new(SystemCacheType::BrowserCache)
    }

    pub fn system_logs() -> Self {
        Self::new(SystemCacheType::SystemLogs)
    }

    pub fn ios_backups() -> Self {
        Self::new(SystemCacheType::IOSBackups)
    }

    pub fn trash() -> Self {
        Self::new(SystemCacheType::Trash)
    }

    pub fn downloads_old() -> Self {
        Self::new(SystemCacheType::DownloadsOld)
    }

    pub fn mail_caches() -> Self {
        Self::new(SystemCacheType::MailCaches)
    }

    pub fn all() -> Self {
        Self::new(SystemCacheType::All)
    }

    fn get_cache_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // Only the SystemCache branch is purely absolute; every other branch
        // derives targets from the user's home. Without a usable absolute home
        // we skip all home-derived pushes so a missing `$HOME` can never yield a
        // relative delete path (GitLab #11). The absolute `/Library/Caches`
        // entries below are unaffected.
        let home = crate::utils::home_dir_checked();

        match self.cache_type {
            SystemCacheType::UserCache => {
                if let Some(home) = &home {
                    paths.push(home.join("Library/Caches"));
                }
            }
            SystemCacheType::SystemCache => {
                paths.push(PathBuf::from("/Library/Caches"));
                paths.push(PathBuf::from("/System/Library/Caches"));
            }
            SystemCacheType::BrowserCache => {
                if let Some(home) = &home {
                    paths.push(home.join("Library/Caches/Google/Chrome"));
                    paths.push(home.join("Library/Caches/com.apple.Safari"));
                    paths.push(home.join("Library/Caches/Firefox"));
                    paths.push(home.join("Library/Caches/Microsoft Edge"));
                    paths.push(home.join("Library/Caches/BraveSoftware"));
                }
            }
            SystemCacheType::SystemLogs => {
                if let Some(home) = &home {
                    paths.push(home.join("Library/Logs"));
                }
                // /var/log requires sudo — only include if accessible
                let var_log = PathBuf::from("/var/log");
                if var_log.exists() {
                    paths.push(var_log);
                }
            }
            SystemCacheType::IOSBackups => {
                if let Some(home) = &home {
                    paths.push(home.join("Library/Application Support/MobileSync/Backup"));
                }
            }
            SystemCacheType::Trash => {
                if let Some(home) = &home {
                    paths.push(home.join(".Trash"));
                }
            }
            SystemCacheType::DownloadsOld => {
                if let Some(home) = &home {
                    paths.push(home.join("Downloads"));
                }
            }
            SystemCacheType::MailCaches => {
                // Only genuine cache locations — NEVER ~/Library/Mail, which is
                // the actual Apple Mail message store (downloaded mail, local
                // "On My Mac" mailboxes, drafts), not a cache.
                if let Some(home) = &home {
                    paths.push(home.join("Library/Caches/com.apple.mail"));
                    paths.push(home.join("Library/Containers/com.apple.mail/Data/Library/Caches"));
                }
            }
            SystemCacheType::All => {
                if let Some(home) = &home {
                    paths.push(home.join("Library/Caches"));
                }
                // Don't include system caches by default (require sudo)
            }
        }

        paths.into_iter().filter(|p| p.exists()).collect()
    }

    fn get_cleanable_items(&self) -> Result<Vec<(PathBuf, u64)>> {
        self.get_cleanable_items_with_age(None)
    }

    fn get_cleanable_items_with_age(&self, min_age_days: Option<u64>) -> Result<Vec<(PathBuf, u64)>> {
        let cache_paths = self.get_cache_paths();
        let mut items = Vec::new();

        for cache_path in cache_paths {
            if !cache_path.exists() {
                continue;
            }

            match self.cache_type {
                // For browser caches, clean the entire directory
                SystemCacheType::BrowserCache => {
                    let (size, _) = calculate_dir_size(&cache_path);
                    items.push((cache_path, size));
                }

                // For iOS backups, enumerate backup subdirectories
                SystemCacheType::IOSBackups => {
                    if let Ok(entries) = std::fs::read_dir(&cache_path) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_dir() {
                                let (size, _) = calculate_dir_size(&path);
                                items.push((path, size));
                            }
                        }
                    }
                }

                // For Trash, enumerate contents (don't delete .Trash itself)
                SystemCacheType::Trash => {
                    if let Ok(entries) = std::fs::read_dir(&cache_path) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            let size = if path.is_dir() {
                                let (s, _) = calculate_dir_size(&path);
                                s
                            } else {
                                path.metadata().map(|m| m.len()).unwrap_or(0)
                            };
                            items.push((path, size));
                        }
                    }
                }

                // For Downloads, filter by file age
                SystemCacheType::DownloadsOld => {
                    let age_days = min_age_days.unwrap_or(30);
                    let cutoff = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                        .saturating_sub(age_days * 24 * 60 * 60);

                    if let Ok(entries) = std::fs::read_dir(&cache_path) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            // Skip hidden files like .DS_Store, .localized
                            if path.file_name()
                                .and_then(|n| n.to_str())
                                .map(|n| n.starts_with('.'))
                                .unwrap_or(false)
                            {
                                continue;
                            }

                            let is_old = path.metadata()
                                .and_then(|m| m.modified())
                                .ok()
                                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                                .map(|d| d.as_secs() < cutoff)
                                .unwrap_or(false);

                            if is_old {
                                let size = if path.is_dir() {
                                    let (s, _) = calculate_dir_size(&path);
                                    s
                                } else {
                                    path.metadata().map(|m| m.len()).unwrap_or(0)
                                };
                                if size > 0 {
                                    items.push((path, size));
                                }
                            }
                        }
                    }
                }

                // For mail caches, clean subdirectories
                SystemCacheType::MailCaches => {
                    let (size, _) = calculate_dir_size(&cache_path);
                    if size > 1024 * 1024 {
                        items.push((cache_path, size));
                    }
                }

                // For logs, enumerate subdirectories
                SystemCacheType::SystemLogs => {
                    for entry in WalkDir::new(&cache_path)
                        .max_depth(1)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|e| e.depth() == 1)
                    {
                        let path = entry.path().to_path_buf();
                        let size = if path.is_dir() {
                            let (s, _) = calculate_dir_size(&path);
                            s
                        } else {
                            path.metadata().map(|m| m.len()).unwrap_or(0)
                        };
                        if size > 1024 * 1024 {
                            items.push((path, size));
                        }
                    }
                }

                // For user/system caches, enumerate subdirectories
                _ => {
                    for entry in WalkDir::new(&cache_path)
                        .max_depth(1)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|e| e.file_type().is_dir() && e.depth() == 1)
                    {
                        let path = entry.path().to_path_buf();
                        let (size, _) = calculate_dir_size(&path);

                        // Skip small caches (less than 1MB)
                        if size > 1024 * 1024 {
                            items.push((path, size));
                        }
                    }
                }
            }
        }

        // Sort by size descending
        items.sort_by(|a, b| b.1.cmp(&a.1));
        Ok(items)
    }
}

impl Cleaner for SystemCacheCleaner {
    fn name(&self) -> &'static str {
        match self.cache_type {
            SystemCacheType::UserCache => "User Cache Cleaner",
            SystemCacheType::SystemCache => "System Cache Cleaner",
            SystemCacheType::BrowserCache => "Browser Cache Cleaner",
            SystemCacheType::SystemLogs => "System Logs Cleaner",
            SystemCacheType::IOSBackups => "iOS Backups Cleaner",
            SystemCacheType::Trash => "Trash Cleaner",
            SystemCacheType::DownloadsOld => "Old Downloads Cleaner",
            SystemCacheType::MailCaches => "Mail Caches Cleaner",
            SystemCacheType::All => "All Caches Cleaner",
        }
    }

    fn description(&self) -> &'static str {
        self.cache_type.description()
    }

    fn clean(&self, options: &CleanOptions) -> Result<CleanResult> {
        let items = self.get_cleanable_items_with_age(options.min_age_days)?;
        let mut result = CleanResult::new();

        for (path, size) in items {
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
                        // Permission errors are expected for some caches
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

    fn estimate(&self) -> Result<u64> {
        let items = self.get_cleanable_items()?;
        Ok(items.iter().map(|(_, size)| *size).sum())
    }
}

// ── SystemScanner ─────────────────────────────────────────────

/// Info about a single system category (for scan-system)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemCategoryInfo {
    pub category: SystemCacheType,
    pub name: String,
    pub description: String,
    pub size_bytes: u64,
    pub item_count: usize,
    pub paths: Vec<SystemCategoryPath>,
}

impl SystemCategoryInfo {
    pub fn size_human(&self) -> String {
        crate::utils::format_size(self.size_bytes)
    }
}

/// A path within a system category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemCategoryPath {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub description: String,
}

/// Result of scanning all system categories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemScanResult {
    pub categories: Vec<SystemCategoryInfo>,
    pub total_size: u64,
    pub scan_duration_ms: u64,
}

impl SystemScanResult {
    pub fn total_size_human(&self) -> String {
        crate::utils::format_size(self.total_size)
    }
}

/// Scanner for system-level cleanable data
pub struct SystemScanner;

impl SystemScanner {
    pub fn new() -> Self {
        Self
    }

    /// Scan all system categories and return sizes without cleaning
    pub fn scan_all(&self) -> Result<SystemScanResult> {
        let start = std::time::Instant::now();
        let mut categories = Vec::new();

        for cat in SystemCacheType::all_categories() {
            if let Ok(info) = self.scan_category(cat) {
                if info.size_bytes > 0 {
                    categories.push(info);
                }
            }
        }

        let total_size = categories.iter().map(|c| c.size_bytes).sum();

        Ok(SystemScanResult {
            categories,
            total_size,
            scan_duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Scan a specific category
    pub fn scan_category(&self, category: SystemCacheType) -> Result<SystemCategoryInfo> {
        let cleaner = SystemCacheCleaner::new(category);
        let items = cleaner.get_cleanable_items()?;

        let paths: Vec<SystemCategoryPath> = items
            .iter()
            .map(|(path, size)| SystemCategoryPath {
                path: path.clone(),
                size_bytes: *size,
                description: path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string()),
            })
            .collect();

        let total_size: u64 = items.iter().map(|(_, s)| *s).sum();

        Ok(SystemCategoryInfo {
            category,
            name: category.name().to_string(),
            description: category.description().to_string(),
            size_bytes: total_size,
            item_count: items.len(),
            paths,
        })
    }
}

// ── Convenience functions ─────────────────────────────────────

/// Get list of cache directories with sizes
pub fn list_caches() -> Result<Vec<(PathBuf, u64)>> {
    let cleaner = SystemCacheCleaner::all();
    cleaner.get_cleanable_items()
}

/// Clean user caches
pub fn clean_user_cache(options: &CleanOptions) -> Result<CleanResult> {
    let cleaner = SystemCacheCleaner::user_cache();
    cleaner.clean(options)
}

/// Clean browser caches
pub fn clean_browser_cache(options: &CleanOptions) -> Result<CleanResult> {
    let cleaner = SystemCacheCleaner::browser_cache();
    cleaner.clean(options)
}

/// Clean system logs
pub fn clean_system_logs(options: &CleanOptions) -> Result<CleanResult> {
    let cleaner = SystemCacheCleaner::system_logs();
    cleaner.clean(options)
}

/// Clean iOS backups
pub fn clean_ios_backups(options: &CleanOptions) -> Result<CleanResult> {
    let cleaner = SystemCacheCleaner::ios_backups();
    cleaner.clean(options)
}

/// Clean trash
pub fn clean_trash(options: &CleanOptions) -> Result<CleanResult> {
    let cleaner = SystemCacheCleaner::trash();
    cleaner.clean(options)
}

/// Clean old downloads
pub fn clean_old_downloads(options: &CleanOptions) -> Result<CleanResult> {
    let cleaner = SystemCacheCleaner::downloads_old();
    cleaner.clean(options)
}

/// Clean mail caches
pub fn clean_mail_caches(options: &CleanOptions) -> Result<CleanResult> {
    let cleaner = SystemCacheCleaner::mail_caches();
    cleaner.clean(options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_cache_cleaner_creation() {
        let cleaner = SystemCacheCleaner::user_cache();
        assert_eq!(cleaner.name(), "User Cache Cleaner");
    }

    #[test]
    fn test_new_cleaner_types() {
        assert_eq!(SystemCacheCleaner::system_logs().name(), "System Logs Cleaner");
        assert_eq!(SystemCacheCleaner::ios_backups().name(), "iOS Backups Cleaner");
        assert_eq!(SystemCacheCleaner::trash().name(), "Trash Cleaner");
        assert_eq!(SystemCacheCleaner::downloads_old().name(), "Old Downloads Cleaner");
        assert_eq!(SystemCacheCleaner::mail_caches().name(), "Mail Caches Cleaner");
    }

    #[test]
    fn test_get_cache_paths() {
        let cleaner = SystemCacheCleaner::user_cache();
        let paths = cleaner.get_cache_paths();
        // Should have at least the user cache path if it exists
        assert!(paths.len() <= 1);
    }

    #[test]
    fn test_all_categories() {
        let cats = SystemCacheType::all_categories();
        assert_eq!(cats.len(), 7);
    }

    #[test]
    fn test_system_scanner() {
        let scanner = SystemScanner::new();
        let result = scanner.scan_all();
        assert!(result.is_ok());
    }

    #[test]
    fn test_mail_caches_never_includes_mail_store() {
        // Regression test (GitLab #5): "Mail Caches" must never resolve to the
        // ~/Library/Mail message store, which holds local-only email and is NOT
        // a cache. We inspect the unfiltered candidate paths so the assertion
        // holds even on machines where ~/Library/Mail does not exist on disk.
        let home = dirs::home_dir().unwrap_or_default();
        let cleaner = SystemCacheCleaner::mail_caches();

        let candidates = vec![
            home.join("Library/Caches/com.apple.mail"),
            home.join("Library/Containers/com.apple.mail/Data/Library/Caches"),
        ];

        // Every candidate must be a cache path, never the mail store.
        let mail_store = home.join("Library/Mail");
        for path in &candidates {
            assert_ne!(*path, mail_store, "MailCaches must not target the mail store");
            assert!(
                !path.ends_with("Library/Mail"),
                "MailCaches path {} must not end in Library/Mail",
                path.display()
            );
        }

        // And the cleaner's actual resolved paths (those that exist on this
        // machine) must likewise never include the mail store.
        for path in cleaner.get_cache_paths() {
            assert!(
                !path.ends_with("Library/Mail"),
                "resolved MailCaches path {} must not end in Library/Mail",
                path.display()
            );
        }
    }
}
