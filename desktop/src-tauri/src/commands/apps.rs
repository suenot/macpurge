//! Application management commands

use macpurge::{AppInfo, AppRemnant, ApplicationScanner};

/// List installed applications
#[tauri::command]
pub fn list_apps() -> Result<Vec<AppInfo>, String> {
    ApplicationScanner::list_apps().map_err(|e| e.to_string())
}

/// Find all files associated with an application
#[tauri::command]
pub fn find_app_files(app_name: String) -> Result<AppFilesResult, String> {
    let files = ApplicationScanner::find_app_files(&app_name).map_err(|e| e.to_string())?;

    let mut total_size: u64 = 0;
    let file_infos: Vec<AppFileInfo> = files
        .iter()
        .map(|p| {
            let size = macpurge::get_size(p);
            total_size += size;
            AppFileInfo {
                path: p.display().to_string(),
                size,
                size_human: format_size(size),
                exists: p.exists(),
                is_dir: p.is_dir(),
            }
        })
        .collect();

    Ok(AppFilesResult {
        app_name,
        files: file_infos,
        total_size,
        total_size_human: format_size(total_size),
    })
}

/// Get data and cache sizes for a specific app (can be slow — call per-app)
#[tauri::command]
pub fn get_app_sizes(app_name: String, bundle_id: Option<String>) -> Result<AppSizesResult, String> {
    let (data_size, cache_size) =
        ApplicationScanner::get_app_sizes(&app_name, bundle_id.as_deref());
    Ok(AppSizesResult {
        app_name,
        data_size_bytes: data_size,
        cache_size_bytes: cache_size,
    })
}

/// Find remnants of uninstalled applications
#[tauri::command]
pub fn find_remnants() -> Result<Vec<AppRemnant>, String> {
    ApplicationScanner::find_remnants().map_err(|e| e.to_string())
}

/// Move an application bundle to the macOS Trash (reversible).
///
/// Takes the exact bundle path (e.g. `/Applications/Docker.app`) rather than a
/// name, so there is no fuzzy/substring matching that could trash the wrong
/// thing. Only `.app` bundles under an `Applications` directory are accepted.
#[tauri::command]
pub fn trash_app(path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);

    if !p.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    if p.extension().and_then(|e| e.to_str()) != Some("app") {
        return Err(format!("Refusing to trash non-.app path: {}", path));
    }
    if !path.contains("/Applications/") {
        return Err(format!("Refusing to trash path outside Applications: {}", path));
    }

    trash::delete(p).map_err(|e| format!("Failed to move to Trash: {}", e))
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[derive(serde::Serialize)]
pub struct AppFilesResult {
    pub app_name: String,
    pub files: Vec<AppFileInfo>,
    pub total_size: u64,
    pub total_size_human: String,
}

#[derive(serde::Serialize)]
pub struct AppFileInfo {
    pub path: String,
    pub size: u64,
    pub size_human: String,
    pub exists: bool,
    pub is_dir: bool,
}

#[derive(serde::Serialize)]
pub struct AppSizesResult {
    pub app_name: String,
    pub data_size_bytes: u64,
    pub cache_size_bytes: u64,
}
