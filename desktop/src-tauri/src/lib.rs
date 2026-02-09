//! MacPurge Desktop Application
//!
//! Tauri 2 application for disk cleanup with system tray support.

mod commands;
mod disk_watcher;
mod tray;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .setup(|app| {
            // Setup system tray
            tray::setup_tray(app)?;

            // Start DiskArbitration watcher for hot-plug events
            disk_watcher::DiskWatcher::new(app.handle().clone()).start();

            // Start background tray updater (reacts to disk events + periodic ticks)
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                tray::start_tray_updater(app_handle);
            });

            // Open devtools in dev mode
            #[cfg(debug_assertions)]
            if let Some(window) = app.get_webview_window("main") {
                window.open_devtools();
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::disk::get_disk_info,
            commands::disk::get_free_space,
            commands::disk::get_free_space_human,
            commands::disk::get_user_disks,
            commands::scan::scan_dev,
            commands::scan::scan_path,
            commands::scan::get_dev_artifact_types,
            commands::clean::clean_artifacts,
            commands::clean::clean_path,
            commands::clean::estimate_cleanup,
            commands::clean::cancel_cleanup,
            commands::clean::is_cleanup_cancelled,
            commands::config::get_config,
            commands::config::save_config,
            commands::config::add_project_dir,
            commands::config::remove_project_dir,
            commands::apps::list_apps,
            commands::apps::find_app_files,
            commands::apps::find_remnants,
            commands::apps::get_app_sizes,
            // Package manager commands
            commands::packages::scan_packages,
            commands::packages::scan_package_manager,
            commands::packages::get_installed_package_managers,
            commands::packages::clean_package_cache,
            // System commands
            commands::system::scan_system,
            commands::system::clean_system_category,
            // DevTools commands
            commands::devtools::scan_devtools,
            commands::devtools::scan_devtool,
            commands::devtools::clean_devtool_cache,
            // Recipe commands
            commands::recipes::list_recipes,
            commands::recipes::run_recipe_cmd,
            commands::recipes::estimate_recipe,
            // Schedule commands
            commands::schedule::get_schedule_status_cmd,
            commands::schedule::enable_schedule_cmd,
            commands::schedule::disable_schedule_cmd,
            commands::schedule::run_schedule_now,
        ])
        .run(tauri::generate_context!())
        .expect("error while running MacPurge");
}
