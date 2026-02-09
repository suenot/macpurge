//! Configuration module for macpurge
//!
//! Handles loading and saving configuration from ~/.macpurge/config.toml

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Directories containing projects to scan
    #[serde(default)]
    pub project_dirs: Vec<PathBuf>,

    /// Directory where archived projects are stored (external disk, cloud folder, etc.)
    #[serde(default)]
    pub archive_dir: Option<PathBuf>,

    /// Scan settings
    #[serde(default)]
    pub scan: ScanConfig,

    /// Cleanup settings
    #[serde(default)]
    pub cleanup: CleanupConfig,

    /// Schedule settings for automated cleanup
    #[serde(default)]
    pub schedule: Option<ScheduleConfig>,
}

/// Scan-related configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    /// Maximum depth for directory scanning
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,

    /// Include hidden directories
    #[serde(default)]
    pub include_hidden: bool,

    /// Minimum size in bytes to report
    #[serde(default)]
    pub min_size_bytes: Option<u64>,

    /// Artifact types to scan for
    #[serde(default = "default_artifact_types")]
    pub artifact_types: Vec<String>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            max_depth: default_max_depth(),
            include_hidden: false,
            min_size_bytes: None,
            artifact_types: default_artifact_types(),
        }
    }
}

fn default_max_depth() -> usize {
    10
}

fn default_artifact_types() -> Vec<String> {
    vec![
        "node_modules".to_string(),
        "target".to_string(),
        ".venv".to_string(),
        "venv".to_string(),
        "__pycache__".to_string(),
        ".next".to_string(),
        ".nuxt".to_string(),
        "dist".to_string(),
        ".gradle".to_string(),
        "build".to_string(),
    ]
}

/// Cleanup-related configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupConfig {
    /// Require confirmation before deletion
    #[serde(default = "default_true")]
    pub confirm_before_delete: bool,

    /// Move to trash instead of permanent delete
    #[serde(default = "default_true")]
    pub use_trash: bool,

    /// Keep a log of all deletions
    #[serde(default = "default_true")]
    pub log_deletions: bool,

    /// Only delete artifacts older than N days
    #[serde(default)]
    pub older_than_days: Option<u32>,
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            confirm_before_delete: true,
            use_trash: true,
            log_deletions: true,
            older_than_days: None,
        }
    }
}

fn default_true() -> bool {
    true
}

/// Schedule configuration for automated cleanup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    /// Whether scheduled cleanup is enabled
    pub enabled: bool,

    /// Cron expression for schedule
    pub cron: String,

    /// Tasks to run on schedule
    #[serde(default)]
    pub tasks: ScheduleTasks,
}

/// Tasks to run on schedule
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScheduleTasks {
    /// Clean dev artifacts
    #[serde(default)]
    pub clean_dev: bool,

    /// Clean package manager caches
    #[serde(default)]
    pub clean_packages: bool,

    /// Clean Docker resources
    #[serde(default)]
    pub clean_docker: bool,

    /// Clean system caches
    #[serde(default)]
    pub clean_system: bool,

    /// Clean browser caches
    #[serde(default)]
    pub clean_browsers: bool,

    /// Empty trash
    #[serde(default)]
    pub empty_trash: bool,
}

/// Get the path to the config directory (~/.macpurge/)
pub fn get_config_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    Ok(home.join(".macpurge"))
}

/// Get the path to the config file (~/.macpurge/config.toml)
pub fn get_config_path() -> Result<PathBuf> {
    Ok(get_config_dir()?.join("config.toml"))
}

/// Get the path to the history log file
pub fn get_history_path() -> Result<PathBuf> {
    Ok(get_config_dir()?.join("history.log"))
}

/// Get the path to the schedule log file
pub fn get_schedule_log_path() -> Result<PathBuf> {
    Ok(get_config_dir()?.join("schedule.log"))
}

/// Load configuration from ~/.macpurge/config.toml
/// Returns default config if file doesn't exist
pub fn load_config() -> Result<Config> {
    let config_path = get_config_path()?;

    if !config_path.exists() {
        return Ok(Config::default());
    }

    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

    let config: Config = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;

    Ok(config)
}

/// Save configuration to ~/.macpurge/config.toml
pub fn save_config(config: &Config) -> Result<()> {
    let config_dir = get_config_dir()?;
    let config_path = get_config_path()?;

    // Create config directory if it doesn't exist
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir)
            .with_context(|| format!("Failed to create config directory: {}", config_dir.display()))?;
    }

    let content = toml::to_string_pretty(config)
        .context("Failed to serialize config")?;

    std::fs::write(&config_path, content)
        .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

    Ok(())
}

/// Initialize config with default values and save to disk
pub fn init_config() -> Result<Config> {
    let config = Config::default();
    save_config(&config)?;
    Ok(config)
}

/// Expand tilde in paths to full home directory path
pub fn expand_path(path: &PathBuf) -> PathBuf {
    if let Some(path_str) = path.to_str() {
        if path_str.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                return home.join(&path_str[2..]);
            }
        }
    }
    path.clone()
}

/// Get expanded project directories from config
pub fn get_project_dirs(config: &Config) -> Vec<PathBuf> {
    config.project_dirs.iter().map(expand_path).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.project_dirs.is_empty());
        assert!(config.cleanup.confirm_before_delete);
        assert!(config.cleanup.use_trash);
    }

    #[test]
    fn test_expand_path() {
        let path = PathBuf::from("~/projects");
        let expanded = expand_path(&path);
        assert!(!expanded.to_str().unwrap().starts_with("~/"));
    }

    #[test]
    fn test_serialize_deserialize() {
        let config = Config {
            project_dirs: vec![PathBuf::from("~/projects"), PathBuf::from("~/work")],
            ..Default::default()
        };

        let toml_str = toml::to_string(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(config.project_dirs, parsed.project_dirs);
    }
}
