use anyhow::Result;
use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::cleaners::CleanResult;
use crate::scanner::{DiskInfo, ScanResult};

/// JSON report structure
#[derive(Debug, Serialize)]
pub struct JsonReport {
    pub generated_at: String,
    pub system_info: SystemInfo,
    pub scan_results: Option<ScanResult>,
    pub disk_usage: Option<Vec<DiskInfo>>,
    pub recommendations: Vec<Recommendation>,
    pub summary: Summary,
}

#[derive(Debug, Serialize)]
pub struct SystemInfo {
    pub os: String,
    pub os_version: String,
    pub hostname: String,
}

#[derive(Debug, Serialize)]
pub struct Recommendation {
    pub category: String,
    pub description: String,
    pub potential_savings: u64,
    pub potential_savings_human: String,
    pub risk_level: String,
    pub command: String,
}

#[derive(Debug, Serialize)]
pub struct Summary {
    pub total_scannable_size: u64,
    pub total_scannable_size_human: String,
    pub total_items: usize,
    pub by_category: Vec<CategorySummary>,
}

#[derive(Debug, Serialize)]
pub struct CategorySummary {
    pub name: String,
    pub count: usize,
    pub size: u64,
    pub size_human: String,
}

impl JsonReport {
    pub fn new() -> Self {
        let now = chrono::Utc::now();

        Self {
            generated_at: now.to_rfc3339(),
            system_info: SystemInfo {
                os: "macOS".to_string(),
                os_version: get_macos_version(),
                hostname: hostname::get()
                    .map(|h| h.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "unknown".to_string()),
            },
            scan_results: None,
            disk_usage: None,
            recommendations: Vec::new(),
            summary: Summary {
                total_scannable_size: 0,
                total_scannable_size_human: "0 B".to_string(),
                total_items: 0,
                by_category: Vec::new(),
            },
        }
    }

    pub fn with_scan_results(mut self, results: ScanResult) -> Self {
        // Generate summary
        let mut by_category: std::collections::HashMap<String, (usize, u64)> =
            std::collections::HashMap::new();

        for entry in &results.entries {
            let category = entry.file_type.display_name().to_string();
            let (count, size) = by_category.entry(category).or_insert((0, 0));
            *count += 1;
            *size += entry.size_bytes;
        }

        self.summary.total_scannable_size = results.total_size;
        self.summary.total_scannable_size_human = crate::utils::format_size(results.total_size);
        self.summary.total_items = results.entries.len();
        self.summary.by_category = by_category
            .into_iter()
            .map(|(name, (count, size))| CategorySummary {
                name,
                count,
                size,
                size_human: crate::utils::format_size(size),
            })
            .collect();

        self.scan_results = Some(results);
        self
    }

    pub fn with_disk_usage(mut self, disks: Vec<DiskInfo>) -> Self {
        self.disk_usage = Some(disks);
        self
    }

    pub fn with_recommendations(mut self, recommendations: Vec<Recommendation>) -> Self {
        self.recommendations = recommendations;
        self
    }

    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let json = self.to_json()?;
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }
}

impl Default for JsonReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a JSON report from scan results
pub fn generate_json_report(scan_result: ScanResult) -> Result<String> {
    let report = JsonReport::new().with_scan_results(scan_result);
    report.to_json()
}

/// Generate a JSON report for clean results
pub fn generate_clean_report(clean_result: &CleanResult) -> Result<String> {
    Ok(serde_json::to_string_pretty(clean_result)?)
}

/// Get macOS version string
fn get_macos_version() -> String {
    std::process::Command::new("sw_vers")
        .arg("-productVersion")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_report_creation() {
        let report = JsonReport::new();
        assert_eq!(report.system_info.os, "macOS");
    }

    #[test]
    fn test_json_serialization() {
        let report = JsonReport::new();
        let json = report.to_json();
        assert!(json.is_ok());
    }
}
