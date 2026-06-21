//! Dev tools scanning and cleaning commands

use macpurge::{
    CleanOptions, CleanResult, Cleaner,
    DevToolType, DevToolScanner, DevToolScanResult, DevToolInfo,
    DevToolsCleaner,
};
use std::path::PathBuf;

#[tauri::command]
pub fn scan_devtools() -> Result<DevToolScanResult, String> {
    let scanner = DevToolScanner::new();
    scanner.scan_all().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn scan_devtool(tool: String) -> Result<DevToolInfo, String> {
    let scanner = DevToolScanner::new();

    let tool_type = match tool.to_lowercase().as_str() {
        "xcode" => DevToolType::Xcode,
        "android" | "android_studio" => DevToolType::AndroidStudio,
        "docker" => DevToolType::Docker,
        "vm" | "vms" | "virtual_machines" => DevToolType::VirtualMachines,
        _ => return Err(format!("Unknown dev tool: {}", tool)),
    };

    scanner.scan_tool(tool_type).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clean_devtool_cache(path: String, dry_run: bool) -> Result<CleanResult, String> {
    // Honor the configured deletion mode (move to Trash vs. permanent delete).
    let use_trash = macpurge::load_config()
        .map(|c| c.cleanup.use_trash)
        .unwrap_or(true);
    let options = CleanOptions {
        dry_run,
        force: true,
        verbose: false,
        use_trash,
        ..Default::default()
    };

    let cleaner = DevToolsCleaner::path(PathBuf::from(&path));
    cleaner.clean(&options).map_err(|e| e.to_string())
}
