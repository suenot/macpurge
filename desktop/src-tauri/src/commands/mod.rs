//! Tauri commands for MacPurge
//!
//! All commands are exposed to the frontend via Tauri IPC.

pub mod disk;
pub mod scan;
pub mod clean;
pub mod config;
pub mod apps;
pub mod packages;
pub mod system;
pub mod devtools;
pub mod ai_models;
pub mod recipes;
pub mod schedule;
pub mod projects;
