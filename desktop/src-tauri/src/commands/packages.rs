//! Package manager scanning commands

use macpurge::{PackageManagerInfo, PackageScanResult, PackageScanner, PkgManagerType};

/// Scan all package managers
#[tauri::command]
pub fn scan_packages() -> Result<PackageScanResult, String> {
    let scanner = PackageScanner::new();
    scanner.scan_all().map_err(|e| e.to_string())
}

/// Scan a specific package manager
#[tauri::command]
pub fn scan_package_manager(manager: String) -> Result<PackageManagerInfo, String> {
    let scanner = PackageScanner::new();

    let pkg_type = match manager.to_lowercase().as_str() {
        "npm" => PkgManagerType::Npm,
        "yarn" => PkgManagerType::Yarn,
        "pnpm" => PkgManagerType::Pnpm,
        "pip" => PkgManagerType::Pip,
        "cargo" => PkgManagerType::Cargo,
        "rustup" => PkgManagerType::Rustup,
        "go" => PkgManagerType::Go,
        "gem" => PkgManagerType::Gem,
        "brew" => PkgManagerType::Brew,
        "composer" => PkgManagerType::Composer,
        _ => return Err(format!("Unknown package manager: {}", manager)),
    };

    scanner.scan_manager(pkg_type).map_err(|e| e.to_string())
}

/// Get list of installed package managers
#[tauri::command]
pub fn get_installed_package_managers() -> Result<Vec<String>, String> {
    let scanner = PackageScanner::new();
    let result = scanner.scan_all().map_err(|e| e.to_string())?;

    Ok(result
        .managers
        .into_iter()
        .filter(|m| m.is_installed)
        .map(|m| m.manager.name().to_string())
        .collect())
}

/// Clean a specific cache path
#[tauri::command]
pub fn clean_package_cache(path: String, dry_run: bool) -> Result<CleanCacheResult, String> {
    use std::path::PathBuf;

    let cache_path = PathBuf::from(&path);

    if !cache_path.exists() {
        return Ok(CleanCacheResult {
            path,
            bytes_freed: 0,
            success: true,
            message: "Path does not exist".to_string(),
        });
    }

    if dry_run {
        let size = macpurge::get_size(&cache_path);
        return Ok(CleanCacheResult {
            path,
            bytes_freed: size,
            success: true,
            message: format!("Would free {} bytes", size),
        });
    }

    // Honor the configured deletion mode (move to Trash vs. permanent delete).
    let use_trash = macpurge::load_config()
        .map(|c| c.cleanup.use_trash)
        .unwrap_or(true);

    match macpurge::remove_path(&cache_path, false, use_trash) {
        Ok(bytes) => Ok(CleanCacheResult {
            path,
            bytes_freed: bytes,
            success: true,
            message: "Cache cleaned successfully".to_string(),
        }),
        Err(e) => Ok(CleanCacheResult {
            path,
            bytes_freed: 0,
            success: false,
            message: e.to_string(),
        }),
    }
}

#[derive(serde::Serialize)]
pub struct CleanCacheResult {
    pub path: String,
    pub bytes_freed: u64,
    pub success: bool,
    pub message: String,
}
