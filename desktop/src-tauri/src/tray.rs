//! System tray functionality for MacPurge
//!
//! Displays free disk space in the macOS menu bar.
//! Shows red indicator when free space is below 10 GB.

use std::sync::mpsc;
use std::time::Duration;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Listener, Manager,
};

use crate::disk_watcher::DisksChangedPayload;

/// Threshold for low disk space warning (10 GB)
const LOW_SPACE_THRESHOLD_GB: f64 = 10.0;

/// Update interval for tray title (30 seconds)
const UPDATE_INTERVAL_SECS: u64 = 30;

/// Setup the system tray with menu and event handlers
pub fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // Create menu items
    let show = MenuItem::with_id(app, "show", "Show MacPurge", true, None::<&str>)?;
    let scan = MenuItem::with_id(app, "scan", "Quick Scan", true, None::<&str>)?;
    let devtools = MenuItem::with_id(app, "devtools", "Toggle DevTools", true, None::<&str>)?;
    let separator = tauri::menu::PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    // Build menu
    let menu = Menu::with_items(app, &[&show, &scan, &devtools, &separator, &quit])?;

    // Create transparent 1x1 pixel icon (text-only tray)
    let transparent_icon = Image::new(&[0u8; 4], 1, 1);

    // Build tray icon with text only (transparent icon)
    let _tray = TrayIconBuilder::with_id("main-tray")
        .icon(transparent_icon)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            handle_menu_event(app, event.id.as_ref());
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    // Set initial title
    update_tray_title(app.handle());

    Ok(())
}

/// Handle menu item clicks
fn handle_menu_event(app: &AppHandle, id: &str) {
    match id {
        "show" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        "scan" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
                // Emit event to trigger scan in frontend
                let _ = window.emit("trigger-scan", ());
            }
        }
        "devtools" => {
            if let Some(window) = app.get_webview_window("main") {
                // Toggle devtools - if open, close; if closed, open
                if window.is_devtools_open() {
                    window.close_devtools();
                } else {
                    window.open_devtools();
                }
            }
        }
        "quit" => {
            app.exit(0);
        }
        _ => {}
    }
}

/// Update tray title with current free disk space
pub fn update_tray_title(app: &AppHandle) {
    if let Ok(Some(disk)) = macpurge::get_system_disk() {
        let available_gb = disk.available_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let is_low = available_gb < LOW_SPACE_THRESHOLD_GB;

        // Format title with warning indicator if low
        let title = if is_low {
            format!("⚠️ {:.1} GB", available_gb)
        } else {
            format!("{:.1} GB", available_gb)
        };

        if let Some(tray) = app.tray_by_id("main-tray") {
            let _ = tray.set_title(Some(&title));

            // Update tooltip with more details
            let tooltip = format!(
                "MacPurge\n{:.1} GB free of {:.1} GB\n{:.1}% used",
                available_gb,
                disk.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
                disk.usage_percent
            );
            let _ = tray.set_tooltip(Some(&tooltip));
        }
    }
}

/// Start the tray updater that responds to both periodic ticks and disk change events.
pub fn start_tray_updater(app: AppHandle) {
    let (tx, rx) = mpsc::channel::<()>();

    // Listen for "disks-changed" events from the DiskArbitration watcher.
    //
    // IMPORTANT: this same event name is also emitted by the update loop below
    // (with reason == "space_update") so the frontend refreshes on periodic
    // ticks. In Tauri 2, `app.emit` delivers to BOTH the webview AND Rust
    // `app.listen` listeners, so without a guard the loop's own periodic emit
    // would re-enter here, push to the channel, trigger another emit, and spin
    // a CPU core (GitLab #7). We therefore IGNORE our own "space_update" emits
    // and only react to REAL mount/unmount events (reason == "mounted" /
    // "unmounted") coming from the disk watcher.
    //
    // The payload arrives as a JSON string; DisksChangedPayload only derives
    // Serialize (it lives in another module), so we do a cheap substring check
    // on the raw payload instead of pulling in a Deserialize impl.
    let tx_event = tx.clone();
    app.listen("disks-changed", move |event| {
        if event.payload().contains("space_update") {
            // Our own periodic refresh emit — do not feed it back into the loop.
            return;
        }
        let _ = tx_event.send(());
    });

    // Periodic ticker thread
    let tx_periodic = tx;
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(UPDATE_INTERVAL_SECS));
        let _ = tx_periodic.send(());
    });

    // Main update loop: waits for either periodic tick or disk-changed event
    loop {
        let _ = rx.recv();
        update_tray_title(&app);
        // Re-emit so frontend can also refresh on tray update
        let _ = app.emit(
            "disks-changed",
            DisksChangedPayload {
                reason: "space_update".to_string(),
            },
        );
    }
}
