//! System scanning and cleaning commands

use macpurge::{
    CleanOptions, CleanResult, SystemCacheCleaner, SystemCacheType,
    SystemScanner, SystemScanResult, Cleaner,
};

#[tauri::command]
pub fn scan_system() -> Result<SystemScanResult, String> {
    let scanner = SystemScanner::new();
    scanner.scan_all().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clean_system_category(
    category: String,
    dry_run: bool,
    min_age_days: Option<u64>,
) -> Result<CleanResult, String> {
    let cache_type = match category.as_str() {
        "user_cache" => SystemCacheType::UserCache,
        "browser_cache" => SystemCacheType::BrowserCache,
        "system_logs" => SystemCacheType::SystemLogs,
        "ios_backups" => SystemCacheType::IOSBackups,
        "trash" => SystemCacheType::Trash,
        "downloads_old" => SystemCacheType::DownloadsOld,
        "mail_caches" => SystemCacheType::MailCaches,
        _ => return Err(format!("Unknown system category: {}", category)),
    };

    // Honor the configured deletion mode (move to Trash vs. permanent delete).
    let use_trash = macpurge::load_config()
        .map(|c| c.cleanup.use_trash)
        .unwrap_or(true);
    let options = CleanOptions {
        dry_run,
        force: true,
        verbose: false,
        min_age_days,
        use_trash,
    };

    let cleaner = SystemCacheCleaner::new(cache_type);
    cleaner.clean(&options).map_err(|e| e.to_string())
}
