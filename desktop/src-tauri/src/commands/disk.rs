//! Disk information commands

use macpurge::DiskInfo;

/// Get information about all mounted disks
#[tauri::command]
pub fn get_disk_info() -> Result<Vec<DiskInfo>, String> {
    macpurge::get_all_disks().map_err(|e| e.to_string())
}

/// Get information about the system disk
#[tauri::command]
pub fn get_free_space() -> Result<DiskInfo, String> {
    macpurge::get_system_disk()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "System disk not found".to_string())
}

/// Get free space as human-readable string
#[tauri::command]
pub fn get_free_space_human() -> Result<FreeSpaceInfo, String> {
    let disk = macpurge::get_system_disk()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "System disk not found".to_string())?;

    let available_gb = disk.available_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    let total_gb = disk.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

    Ok(FreeSpaceInfo {
        available_bytes: disk.available_bytes,
        total_bytes: disk.total_bytes,
        available_gb,
        total_gb,
        available_human: format!("{:.1} GB", available_gb),
        usage_percent: disk.usage_percent,
        is_low: available_gb < 10.0,
    })
}

/// Get all user-accessible disks with type classification
#[tauri::command]
pub fn get_user_disks() -> Result<Vec<DiskInfo>, String> {
    macpurge::get_user_disks().map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
pub struct FreeSpaceInfo {
    pub available_bytes: u64,
    pub total_bytes: u64,
    pub available_gb: f64,
    pub total_gb: f64,
    pub available_human: String,
    pub usage_percent: f64,
    pub is_low: bool,
}
