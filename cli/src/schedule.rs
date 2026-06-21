//! Schedule module — macOS LaunchAgent for automated cleanup

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

use crate::cleaners::{CleanOptions, CleanResult};
use crate::config::{self, ScheduleTasks};

const LAUNCH_AGENT_LABEL: &str = "com.macpurge.scheduled-cleanup";

/// Status of the scheduled cleanup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleStatus {
    pub enabled: bool,
    pub cron: String,
    pub plist_path: String,
    pub plist_exists: bool,
    pub is_loaded: bool,
    pub tasks: ScheduleTasks,
    pub last_run: Option<String>,
    pub next_run: Option<String>,
}

/// Result of running scheduled tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleRunResult {
    pub result: CleanResult,
    pub timestamp: String,
    pub tasks_run: Vec<String>,
}

/// Get the LaunchAgent plist path
pub fn get_launch_agent_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    Ok(home.join(format!("Library/LaunchAgents/{}.plist", LAUNCH_AGENT_LABEL)))
}

/// Generate a LaunchAgent plist from schedule config
pub fn generate_plist(cron: &str, macpurge_path: &str) -> String {
    let calendar = cron_to_calendar_interval(cron);

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{path}</string>
        <string>schedule</string>
        <string>run-now</string>
    </array>
{calendar}
    <key>StandardOutPath</key>
    <string>{log_dir}/schedule-stdout.log</string>
    <key>StandardErrorPath</key>
    <string>{log_dir}/schedule-stderr.log</string>
</dict>
</plist>"#,
        label = LAUNCH_AGENT_LABEL,
        path = macpurge_path,
        calendar = calendar,
        log_dir = dirs::home_dir()
            .unwrap_or_default()
            .join(".macpurge")
            .display(),
    )
}

/// Convert simple cron expression to plist CalendarInterval
/// Supports: "daily", "weekly", "monthly", or "H M * * *" format
fn cron_to_calendar_interval(cron: &str) -> String {
    match cron.to_lowercase().as_str() {
        "daily" => {
            format!(
                "    <key>StartCalendarInterval</key>\n    <dict>\n        <key>Hour</key>\n        <integer>3</integer>\n        <key>Minute</key>\n        <integer>0</integer>\n    </dict>"
            )
        }
        "weekly" => {
            format!(
                "    <key>StartCalendarInterval</key>\n    <dict>\n        <key>Weekday</key>\n        <integer>0</integer>\n        <key>Hour</key>\n        <integer>3</integer>\n        <key>Minute</key>\n        <integer>0</integer>\n    </dict>"
            )
        }
        "monthly" => {
            format!(
                "    <key>StartCalendarInterval</key>\n    <dict>\n        <key>Day</key>\n        <integer>1</integer>\n        <key>Hour</key>\n        <integer>3</integer>\n        <key>Minute</key>\n        <integer>0</integer>\n    </dict>"
            )
        }
        _ => {
            // Try to parse "M H * * *" (minute, hour)
            let parts: Vec<&str> = cron.split_whitespace().collect();
            if parts.len() >= 2 {
                let minute: u32 = parts[0].parse().unwrap_or(0);
                let hour: u32 = parts[1].parse().unwrap_or(3);
                format!(
                    "    <key>StartCalendarInterval</key>\n    <dict>\n        <key>Hour</key>\n        <integer>{}</integer>\n        <key>Minute</key>\n        <integer>{}</integer>\n    </dict>",
                    hour, minute
                )
            } else {
                // Default: daily at 3am
                format!(
                    "    <key>StartCalendarInterval</key>\n    <dict>\n        <key>Hour</key>\n        <integer>3</integer>\n        <key>Minute</key>\n        <integer>0</integer>\n    </dict>"
                )
            }
        }
    }
}

/// Enable scheduled cleanup by writing plist and loading it
pub fn enable_schedule(cron: &str, tasks: &ScheduleTasks) -> Result<()> {
    let plist_path = get_launch_agent_path()?;

    // Ensure LaunchAgents directory exists
    if let Some(parent) = plist_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Find macpurge binary path
    let macpurge_path = std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("/usr/local/bin/macpurge"));

    let plist_content = generate_plist(cron, &macpurge_path.display().to_string());
    std::fs::write(&plist_path, &plist_content)
        .with_context(|| format!("Failed to write plist: {}", plist_path.display()))?;

    // Save schedule config
    let mut cfg = config::load_config()?;
    cfg.schedule = Some(config::ScheduleConfig {
        enabled: true,
        cron: cron.to_string(),
        tasks: tasks.clone(),
    });
    config::save_config(&cfg)?;

    // Load the LaunchAgent
    let output = Command::new("launchctl")
        .args(["load", &plist_path.display().to_string()])
        .output()
        .context("Failed to run launchctl load")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // "already loaded" is not an error
        if !stderr.contains("already loaded") {
            anyhow::bail!("launchctl load failed: {}", stderr);
        }
    }

    Ok(())
}

/// Disable scheduled cleanup
pub fn disable_schedule() -> Result<()> {
    let plist_path = get_launch_agent_path()?;

    // Unload if loaded
    if plist_path.exists() {
        let _ = Command::new("launchctl")
            .args(["unload", &plist_path.display().to_string()])
            .output();

        std::fs::remove_file(&plist_path)?;
    }

    // Update config
    let mut cfg = config::load_config()?;
    if let Some(ref mut schedule) = cfg.schedule {
        schedule.enabled = false;
    }
    config::save_config(&cfg)?;

    Ok(())
}

/// Get current schedule status
pub fn get_schedule_status() -> Result<ScheduleStatus> {
    let plist_path = get_launch_agent_path()?;
    let plist_exists = plist_path.exists();

    let cfg = config::load_config()?;
    let schedule = cfg.schedule.clone().unwrap_or(config::ScheduleConfig {
        enabled: false,
        cron: "daily".to_string(),
        tasks: ScheduleTasks::default(),
    });

    // Check if loaded via launchctl list
    let is_loaded = if plist_exists {
        Command::new("launchctl")
            .args(["list", LAUNCH_AGENT_LABEL])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    } else {
        false
    };

    // Read last run from log
    let last_run = config::get_schedule_log_path()
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|content| content.lines().last().map(String::from));

    Ok(ScheduleStatus {
        enabled: schedule.enabled && is_loaded,
        cron: schedule.cron,
        plist_path: plist_path.display().to_string(),
        plist_exists,
        is_loaded,
        tasks: schedule.tasks,
        last_run,
        next_run: None, // Would require parsing CalendarInterval — skip for now
    })
}

/// Run scheduled tasks now (called by LaunchAgent or manually)
pub fn run_scheduled_tasks() -> Result<ScheduleRunResult> {
    let cfg = config::load_config()?;
    let tasks = cfg.schedule
        .as_ref()
        .map(|s| s.tasks.clone())
        .unwrap_or_default();

    let options = CleanOptions {
        dry_run: false,
        force: true,
        verbose: false,
        use_trash: cfg.cleanup.use_trash,
        ..Default::default()
    };

    let mut total = CleanResult::new();
    let mut tasks_run = Vec::new();

    if tasks.clean_dev {
        let dirs = config::get_project_dirs(&cfg);
        for dir in dirs {
            if dir.exists() {
                if let Ok(r) = crate::cleaners::clean_all_dev_artifacts(&dir, &options) {
                    total.merge(r);
                }
            }
        }
        tasks_run.push("Dev artifacts".to_string());
    }

    if tasks.clean_packages {
        for cleaner_fn in [
            crate::cleaners::clean_npm_cache,
            crate::cleaners::clean_cargo_cache,
            crate::cleaners::clean_pip_cache,
            crate::cleaners::clean_homebrew_cache,
        ] {
            if let Ok(r) = cleaner_fn(&options) {
                total.merge(r);
            }
        }
        tasks_run.push("Package caches".to_string());
    }

    if tasks.clean_docker {
        if let Ok(r) = crate::cleaners::clean_docker_dangling(&options) {
            total.merge(r);
        }
        tasks_run.push("Docker".to_string());
    }

    if tasks.clean_system {
        if let Ok(r) = crate::cleaners::clean_system_logs(&options) {
            total.merge(r);
        }
        tasks_run.push("System logs".to_string());
    }

    if tasks.clean_browsers {
        if let Ok(r) = crate::cleaners::clean_browser_cache(&options) {
            total.merge(r);
        }
        tasks_run.push("Browser caches".to_string());
    }

    if tasks.empty_trash {
        if let Ok(r) = crate::cleaners::clean_trash(&options) {
            total.merge(r);
        }
        tasks_run.push("Trash".to_string());
    }

    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // Log the run
    if let Ok(log_path) = config::get_schedule_log_path() {
        let log_line = format!(
            "{} | Freed {} | Tasks: {}\n",
            timestamp,
            crate::utils::format_size(total.bytes_freed),
            tasks_run.join(", ")
        );
        // Append to log (create if doesn't exist)
        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .and_then(|mut f| {
                use std::io::Write;
                f.write_all(log_line.as_bytes())
            });
    }

    // Send macOS notification
    let notification_msg = format!(
        "MacPurge freed {}. Tasks: {}",
        crate::utils::format_size(total.bytes_freed),
        tasks_run.join(", ")
    );
    let _ = Command::new("osascript")
        .args([
            "-e",
            &format!(
                "display notification \"{}\" with title \"MacPurge Scheduled Cleanup\"",
                notification_msg
            ),
        ])
        .output();

    Ok(ScheduleRunResult {
        result: total,
        timestamp,
        tasks_run,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_to_calendar_daily() {
        let result = cron_to_calendar_interval("daily");
        assert!(result.contains("Hour"));
        assert!(result.contains("<integer>3</integer>"));
    }

    #[test]
    fn test_cron_to_calendar_weekly() {
        let result = cron_to_calendar_interval("weekly");
        assert!(result.contains("Weekday"));
    }

    #[test]
    fn test_cron_to_calendar_custom() {
        let result = cron_to_calendar_interval("30 2 * * *");
        assert!(result.contains("<integer>2</integer>"));  // Hour
        assert!(result.contains("<integer>30</integer>")); // Minute
    }

    #[test]
    fn test_launch_agent_path() {
        let path = get_launch_agent_path();
        assert!(path.is_ok());
        assert!(path.unwrap().display().to_string().contains("LaunchAgents"));
    }

    #[test]
    fn test_generate_plist() {
        let plist = generate_plist("daily", "/usr/local/bin/macpurge");
        assert!(plist.contains(LAUNCH_AGENT_LABEL));
        assert!(plist.contains("schedule"));
        assert!(plist.contains("run-now"));
    }
}
