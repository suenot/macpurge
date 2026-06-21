//! Configuration commands

use std::path::PathBuf;

/// Get current configuration
#[tauri::command]
pub fn get_config() -> Result<ConfigResponse, String> {
    let config = macpurge::load_config().map_err(|e| e.to_string())?;
    let config_path = macpurge::get_config_path().map_err(|e| e.to_string())?;

    // Expand paths for display
    let project_dirs: Vec<ProjectDir> = config
        .project_dirs
        .iter()
        .map(|p| {
            let expanded = macpurge::expand_path(p);
            ProjectDir {
                path: p.display().to_string(),
                expanded: expanded.display().to_string(),
                exists: expanded.exists(),
            }
        })
        .collect();

    let archive_dir = config.archive_dir.as_ref().map(|p| {
        macpurge::expand_path(p).display().to_string()
    });

    // Global exclude list (folders never scanned by any feature).
    let excluded_dirs: Vec<ProjectDir> = config
        .excluded_dirs
        .iter()
        .map(|p| {
            let expanded = macpurge::expand_path(p);
            ProjectDir {
                path: p.display().to_string(),
                expanded: expanded.display().to_string(),
                exists: expanded.exists(),
            }
        })
        .collect();

    Ok(ConfigResponse {
        config_path: config_path.display().to_string(),
        project_dirs,
        excluded_dirs,
        scan: ScanConfigResponse {
            max_depth: config.scan.max_depth,
            include_hidden: config.scan.include_hidden,
            artifact_types: config.scan.artifact_types.clone(),
        },
        cleanup: CleanupConfigResponse {
            confirm_before_delete: config.cleanup.confirm_before_delete,
            use_trash: config.cleanup.use_trash,
            log_deletions: config.cleanup.log_deletions,
            older_than_days: config.cleanup.older_than_days,
        },
        schedule_enabled: config.schedule.as_ref().map(|s| s.enabled).unwrap_or(false),
        archive_dir,
        onboarding_completed: config.onboarding_completed,
    })
}

/// Save configuration
#[tauri::command]
pub fn save_config(config: ConfigInput) -> Result<(), String> {
    let mut current = macpurge::load_config().unwrap_or_default();

    // Update project dirs
    current.project_dirs = config
        .project_dirs
        .into_iter()
        .map(PathBuf::from)
        .collect();

    // Update scan settings
    current.scan.max_depth = config.scan.max_depth;
    current.scan.include_hidden = config.scan.include_hidden;

    // Update cleanup settings
    current.cleanup.confirm_before_delete = config.cleanup.confirm_before_delete;
    current.cleanup.use_trash = config.cleanup.use_trash;

    macpurge::save_config(&current).map_err(|e| e.to_string())
}

/// Add a project directory
#[tauri::command]
pub fn add_project_dir(path: String) -> Result<ConfigResponse, String> {
    let mut config = macpurge::load_config().unwrap_or_default();
    let path_buf = PathBuf::from(&path);

    // Check if already exists
    let expanded = macpurge::expand_path(&path_buf);
    let already_exists = config.project_dirs.iter().any(|p| {
        let p_expanded = macpurge::expand_path(p);
        p_expanded == expanded
    });

    if already_exists {
        return Err("Directory already in configuration".to_string());
    }

    // Check if directory exists
    if !expanded.exists() {
        return Err(format!("Directory does not exist: {}", expanded.display()));
    }

    config.project_dirs.push(path_buf);
    macpurge::save_config(&config).map_err(|e| e.to_string())?;

    get_config()
}

/// Remove a project directory
#[tauri::command]
pub fn remove_project_dir(path: String) -> Result<ConfigResponse, String> {
    let mut config = macpurge::load_config().map_err(|e| e.to_string())?;
    let path_buf = PathBuf::from(&path);
    let expanded = macpurge::expand_path(&path_buf);

    let before_len = config.project_dirs.len();
    config.project_dirs.retain(|p| {
        let p_expanded = macpurge::expand_path(p);
        p_expanded != expanded && *p != path_buf
    });

    if config.project_dirs.len() == before_len {
        return Err("Directory not found in configuration".to_string());
    }

    macpurge::save_config(&config).map_err(|e| e.to_string())?;

    get_config()
}

#[derive(serde::Serialize)]
pub struct ConfigResponse {
    pub config_path: String,
    pub project_dirs: Vec<ProjectDir>,
    pub excluded_dirs: Vec<ProjectDir>,
    pub scan: ScanConfigResponse,
    pub cleanup: CleanupConfigResponse,
    pub schedule_enabled: bool,
    pub archive_dir: Option<String>,
    pub onboarding_completed: bool,
}

/// Add a folder to the global exclude list (never scanned by any feature).
#[tauri::command]
pub fn add_excluded_dir(path: String) -> Result<ConfigResponse, String> {
    let mut config = macpurge::load_config().unwrap_or_default();
    let path_buf = PathBuf::from(&path);
    let expanded = macpurge::expand_path(&path_buf);

    let already = config.excluded_dirs.iter().any(|p| macpurge::expand_path(p) == expanded);
    if already {
        return Err("Folder already excluded".to_string());
    }

    config.excluded_dirs.push(path_buf);
    macpurge::save_config(&config).map_err(|e| e.to_string())?;
    get_config()
}

/// Remove a folder from the global exclude list.
#[tauri::command]
pub fn remove_excluded_dir(path: String) -> Result<ConfigResponse, String> {
    let mut config = macpurge::load_config().map_err(|e| e.to_string())?;
    let path_buf = PathBuf::from(&path);
    let expanded = macpurge::expand_path(&path_buf);

    let before = config.excluded_dirs.len();
    config.excluded_dirs.retain(|p| macpurge::expand_path(p) != expanded && *p != path_buf);
    if config.excluded_dirs.len() == before {
        return Err("Folder not found in exclude list".to_string());
    }

    macpurge::save_config(&config).map_err(|e| e.to_string())?;
    get_config()
}

/// Persist whether the first-launch permissions onboarding has been completed.
#[tauri::command]
pub fn set_onboarding_completed(value: bool) -> Result<(), String> {
    let mut config = macpurge::load_config().unwrap_or_default();
    config.onboarding_completed = value;
    macpurge::save_config(&config).map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
pub struct ProjectDir {
    pub path: String,
    pub expanded: String,
    pub exists: bool,
}

#[derive(serde::Serialize)]
pub struct ScanConfigResponse {
    pub max_depth: usize,
    pub include_hidden: bool,
    pub artifact_types: Vec<String>,
}

#[derive(serde::Serialize)]
pub struct CleanupConfigResponse {
    pub confirm_before_delete: bool,
    pub use_trash: bool,
    pub log_deletions: bool,
    pub older_than_days: Option<u32>,
}

/// Set archive directory
#[tauri::command]
pub fn set_archive_dir(path: String) -> Result<ConfigResponse, String> {
    let mut config = macpurge::load_config().map_err(|e| e.to_string())?;
    let path_buf = PathBuf::from(&path);
    let expanded = macpurge::expand_path(&path_buf);

    if !expanded.exists() {
        return Err(format!("Directory does not exist: {}", expanded.display()));
    }

    config.archive_dir = Some(path_buf);
    macpurge::save_config(&config).map_err(|e| e.to_string())?;
    get_config()
}

/// Clear archive directory
#[tauri::command]
pub fn clear_archive_dir() -> Result<ConfigResponse, String> {
    let mut config = macpurge::load_config().map_err(|e| e.to_string())?;
    config.archive_dir = None;
    macpurge::save_config(&config).map_err(|e| e.to_string())?;
    get_config()
}

#[derive(serde::Deserialize)]
pub struct ConfigInput {
    pub project_dirs: Vec<String>,
    pub scan: ScanConfigInput,
    pub cleanup: CleanupConfigInput,
}

#[derive(serde::Deserialize)]
pub struct ScanConfigInput {
    pub max_depth: usize,
    pub include_hidden: bool,
}

#[derive(serde::Deserialize)]
pub struct CleanupConfigInput {
    pub confirm_before_delete: bool,
    pub use_trash: bool,
}
