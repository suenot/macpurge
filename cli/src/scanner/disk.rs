use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Type of disk connection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiskType {
    System,
    Internal,
    External,
    Network,
}

/// Information about a disk/volume
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: PathBuf,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
    pub usage_percent: f64,
    pub disk_type: DiskType,
    pub fs_type: String,
}

impl DiskInfo {
    /// Get human-readable total size
    pub fn total_human(&self) -> String {
        crate::utils::format_size(self.total_bytes)
    }

    /// Get human-readable available size
    pub fn available_human(&self) -> String {
        crate::utils::format_size(self.available_bytes)
    }

    /// Get human-readable used size
    pub fn used_human(&self) -> String {
        crate::utils::format_size(self.used_bytes)
    }
}

/// Classify a disk based on mount point and filesystem type
fn classify_disk(mount_point: &Path, fs_type: &str) -> DiskType {
    if mount_point == Path::new("/") {
        return DiskType::System;
    }
    match fs_type {
        "smbfs" | "nfs" | "afpfs" | "webdav" | "cifs" => DiskType::Network,
        _ => {
            if mount_point.starts_with("/Volumes") {
                DiskType::External
            } else {
                DiskType::Internal
            }
        }
    }
}

/// Parse `mount` output to build a map of mount_point → filesystem type
fn get_fs_types() -> HashMap<String, String> {
    let mut map = HashMap::new();

    let output = Command::new("mount").output();
    let output = match output {
        Ok(o) => o,
        Err(_) => return map,
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Format: "<device> on <path> (<fs_type>, <options...>)"
    for line in stdout.lines() {
        if let Some(on_pos) = line.find(" on ") {
            let rest = &line[on_pos + 4..];
            // Find the last " (" to split path from options
            if let Some(paren_pos) = rest.rfind(" (") {
                let mount_path = &rest[..paren_pos];
                let options = &rest[paren_pos + 2..];
                // fs_type is the first comma-separated value inside parens
                let fs_type = options
                    .trim_end_matches(')')
                    .split(',')
                    .next()
                    .unwrap_or("")
                    .trim();
                map.insert(mount_path.to_string(), fs_type.to_string());
            }
        }
    }

    map
}

/// Get mount points of DMG disk images via `hdiutil info`
fn get_dmg_mount_points() -> HashSet<String> {
    let mut mounts = HashSet::new();

    let output = match Command::new("hdiutil").args(["info"]).output() {
        Ok(o) => o,
        Err(_) => return mounts,
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let trimmed = line.trim();
        // Disk image entries have lines like:
        //   /dev/disk4s1    Apple_HFS    /Volumes/AppName
        if trimmed.starts_with("/dev/disk") {
            let parts: Vec<&str> = trimmed.split('\t').collect();
            if let Some(last) = parts.last() {
                let mount_point = last.trim();
                if !mount_point.is_empty() && mount_point.starts_with('/') {
                    mounts.insert(mount_point.to_string());
                }
            }
        }
    }

    mounts
}

/// Get list of all mounted disks
pub fn get_all_disks() -> Result<Vec<DiskInfo>> {
    let fs_types = get_fs_types();
    let dmg_mounts = get_dmg_mount_points();

    let output = Command::new("df")
        .args(["-k"])
        .output()
        .context("Failed to execute df command")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut disks = Vec::new();

    for line in stdout.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 6 {
            let mount_point = parts[8..].join(" ");

            // Skip system volumes, special mounts, and DMG disk images
            if mount_point.starts_with("/System")
                || mount_point.starts_with("/private/var/vm")
                || mount_point == "/dev"
                || dmg_mounts.contains(&mount_point)
            {
                continue;
            }

            let total_kb: u64 = parts[1].parse().unwrap_or(0);
            let used_kb: u64 = parts[2].parse().unwrap_or(0);
            let available_kb: u64 = parts[3].parse().unwrap_or(0);

            let total_bytes = total_kb * 1024;
            let used_bytes = used_kb * 1024;
            let available_bytes = available_kb * 1024;

            // On APFS, "used" only shows this volume's usage, not container total
            // Calculate based on (total - available) for accurate user-facing percentage
            let usage_percent = if total_bytes > 0 {
                ((total_bytes - available_bytes) as f64 / total_bytes as f64) * 100.0
            } else {
                0.0
            };

            // Get volume name from mount point
            let name = PathBuf::from(&mount_point)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&mount_point)
                .to_string();

            let fs_type = fs_types
                .get(&mount_point)
                .cloned()
                .unwrap_or_default();
            let disk_type = classify_disk(Path::new(&mount_point), &fs_type);

            disks.push(DiskInfo {
                name,
                mount_point: PathBuf::from(mount_point),
                total_bytes,
                available_bytes,
                used_bytes,
                usage_percent,
                disk_type,
                fs_type,
            });
        }
    }

    Ok(disks)
}

/// Get disk info for a specific path
pub fn get_disk_for_path(path: &PathBuf) -> Result<Option<DiskInfo>> {
    let disks = get_all_disks()?;

    // Find the disk with the longest matching mount point
    let mut best_match: Option<&DiskInfo> = None;
    let mut best_len = 0;

    for disk in &disks {
        if path.starts_with(&disk.mount_point) {
            let len = disk.mount_point.as_os_str().len();
            if len > best_len {
                best_len = len;
                best_match = Some(disk);
            }
        }
    }

    Ok(best_match.cloned())
}

/// Get the main system disk (usually /)
pub fn get_system_disk() -> Result<Option<DiskInfo>> {
    let disks = get_all_disks()?;
    Ok(disks.into_iter().find(|d| d.mount_point == PathBuf::from("/")))
}

/// Get all user-accessible disks (excluding system volumes)
pub fn get_user_disks() -> Result<Vec<DiskInfo>> {
    let disks = get_all_disks()?;
    Ok(disks
        .into_iter()
        .filter(|d| {
            d.mount_point == PathBuf::from("/")
                || d.mount_point.starts_with("/Volumes")
                || d.mount_point.starts_with("/Users")
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_disks() {
        let disks = get_all_disks();
        assert!(disks.is_ok());
        let disks = disks.unwrap();
        assert!(!disks.is_empty());
    }

    #[test]
    fn test_get_system_disk() {
        let disk = get_system_disk();
        assert!(disk.is_ok());
    }

    #[test]
    fn test_user_disks_has_system() {
        let disks = get_user_disks().unwrap();
        assert!(disks.iter().any(|d| d.disk_type == DiskType::System));
    }
}
