//! Schedule commands

use macpurge::{
    ScheduleStatus, ScheduleRunResult as ScheduleRun,
    enable_schedule, disable_schedule, get_schedule_status, run_scheduled_tasks,
};
use macpurge::ScheduleTasks;

#[tauri::command]
pub fn get_schedule_status_cmd() -> Result<ScheduleStatus, String> {
    get_schedule_status().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn enable_schedule_cmd(cron: String, tasks: ScheduleTasks) -> Result<(), String> {
    enable_schedule(&cron, &tasks).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn disable_schedule_cmd() -> Result<(), String> {
    disable_schedule().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn run_schedule_now() -> Result<ScheduleRun, String> {
    run_scheduled_tasks().map_err(|e| e.to_string())
}
