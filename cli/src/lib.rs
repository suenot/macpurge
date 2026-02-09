//! macpurge - CLI tool for cleaning macOS disk space
//!
//! A comprehensive tool for analyzing and cleaning disk space on macOS,
//! with special focus on development artifacts and package manager caches.

pub mod scanner;
pub mod cleaners;
pub mod reporters;
pub mod claude;
pub mod utils;
pub mod config;
pub mod recipes;
pub mod schedule;

pub use scanner::{
    DiskInfo, DiskType, DirectoryScanner, FileType, ScanOptions, ScanResult, ScannedEntry,
    calculate_dir_size, find_directories, get_all_disks, get_disk_for_path,
    get_size, get_system_disk, get_user_disks,
    // Package scanner
    CachePath, GlobalPackage, PackageManagerInfo, PackageScanResult, PackageScanner,
    PkgManagerType,
    // DevTools scanner
    DevToolType, DevToolCachePath, DevToolInfo, DevToolScanResult, DevToolScanner,
    // Project scanner
    ProjectType, ProjectInfo, ProjectScanner,
    ArchivedProjectEntry, ArchiveManifest,
    load_archive_manifest, save_archive_manifest,
    copy_dir_recursive, unique_dest_path,
};

pub use cleaners::{
    AppInfo, AppRemnant, ApplicationScanner, CleanOptions, CleanResult, Cleaner,
    DevArtifactsCleaner, DevArtifactType, PackageManager, PackageManagerCleaner,
    SystemCacheCleaner, SystemCacheType,
    // System scanner
    SystemScanner, SystemScanResult, SystemCategoryInfo, SystemCategoryPath,
    // DevTools cleaner
    DevToolsCleaner, DevToolCleanType,
    clean_all_dev_artifacts, clean_app_remnants, clean_browser_cache,
    clean_cargo_cache, clean_homebrew_cache, clean_node_modules, clean_npm_cache,
    clean_pip_cache, clean_python_venv, clean_rust_target, clean_user_cache,
    clean_system_logs, clean_ios_backups, clean_trash, clean_old_downloads,
    clean_mail_caches, clean_xcode_derived_data, clean_docker_dangling,
    clean_gradle_cache, clean_devtool_path,
    list_caches, list_package_manager_caches, remove_path,
};

pub use reporters::{
    JsonReport, MarkdownReport, Recommendation,
    display_apps, display_clean_results, display_disk_info, display_remnants,
    display_scan_results, generate_claude_report, generate_clean_report,
    generate_json_report, generate_summary,
    confirm, error, info, success, warn,
};

pub use claude::ClaudeIntegration;

pub use config::{
    Config, ScanConfig, CleanupConfig, ScheduleConfig, ScheduleTasks,
    get_config_dir, get_config_path, get_history_path, get_schedule_log_path,
    load_config, save_config, init_config, expand_path, get_project_dirs,
};

pub use recipes::{
    Recipe, RecipeStep, RecipeAction, RecipeRunResult, StepResult,
    get_builtin_recipes, run_recipe,
};

pub use schedule::{
    ScheduleStatus, ScheduleRunResult,
    get_launch_agent_path, enable_schedule, disable_schedule,
    get_schedule_status, run_scheduled_tasks,
};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");
