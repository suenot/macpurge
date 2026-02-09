use anyhow::Result;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::cleaners::{list_package_manager_caches, ApplicationScanner};
use crate::scanner::{DiskInfo, ScanResult};

/// Generate a Markdown report for Claude analysis
pub struct MarkdownReport {
    content: String,
}

impl MarkdownReport {
    pub fn new() -> Self {
        Self {
            content: String::new(),
        }
    }

    pub fn add_header(&mut self, title: &str) {
        self.content.push_str(&format!("# {}\n\n", title));
        self.content.push_str(&format!(
            "Generated: {}\n\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        ));
    }

    pub fn add_section(&mut self, title: &str) {
        self.content.push_str(&format!("\n## {}\n\n", title));
    }

    pub fn add_subsection(&mut self, title: &str) {
        self.content.push_str(&format!("\n### {}\n\n", title));
    }

    pub fn add_paragraph(&mut self, text: &str) {
        self.content.push_str(text);
        self.content.push_str("\n\n");
    }

    pub fn add_table(&mut self, headers: &[&str], rows: Vec<Vec<String>>) {
        // Header row
        self.content.push_str("| ");
        self.content.push_str(&headers.join(" | "));
        self.content.push_str(" |\n");

        // Separator row
        self.content.push_str("| ");
        self.content.push_str(
            &headers
                .iter()
                .map(|_| "---")
                .collect::<Vec<_>>()
                .join(" | "),
        );
        self.content.push_str(" |\n");

        // Data rows
        for row in rows {
            self.content.push_str("| ");
            self.content.push_str(&row.join(" | "));
            self.content.push_str(" |\n");
        }
        self.content.push('\n');
    }

    pub fn add_code_block(&mut self, code: &str, lang: &str) {
        self.content.push_str(&format!("```{}\n", lang));
        self.content.push_str(code);
        self.content.push_str("\n```\n\n");
    }

    pub fn add_list(&mut self, items: &[String]) {
        for item in items {
            self.content.push_str(&format!("- {}\n", item));
        }
        self.content.push('\n');
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let mut file = File::create(path)?;
        file.write_all(self.content.as_bytes())?;
        Ok(())
    }
}

impl Default for MarkdownReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a comprehensive Markdown report for Claude analysis
pub fn generate_claude_report(
    scan_result: Option<&ScanResult>,
    disk_info: Option<&[DiskInfo]>,
) -> Result<String> {
    let mut report = MarkdownReport::new();

    report.add_header("macOS Disk Space Analysis Report");

    // System Overview
    if let Some(disks) = disk_info {
        report.add_section("Disk Usage Overview");

        let rows: Vec<Vec<String>> = disks
            .iter()
            .map(|d| {
                vec![
                    d.name.clone(),
                    d.mount_point.display().to_string(),
                    d.total_human(),
                    d.used_human(),
                    d.available_human(),
                    format!("{:.1}%", d.usage_percent),
                ]
            })
            .collect();

        report.add_table(
            &["Name", "Mount Point", "Total", "Used", "Available", "Usage"],
            rows,
        );
    }

    // Scan Results
    if let Some(result) = scan_result {
        report.add_section("Development Artifacts Found");

        report.add_paragraph(&format!(
            "Scanned: `{}`\n\nTotal cleanable: **{}** ({} items)",
            result.root_path.display(),
            result.total_size_human(),
            result.entries.len()
        ));

        // Group by type
        report.add_subsection("By Category");

        let mut by_type: std::collections::HashMap<String, (usize, u64)> =
            std::collections::HashMap::new();

        for entry in &result.entries {
            let type_name = entry.file_type.display_name().to_string();
            let (count, size) = by_type.entry(type_name).or_insert((0, 0));
            *count += 1;
            *size += entry.size_bytes;
        }

        let mut type_rows: Vec<(String, usize, u64)> = by_type
            .into_iter()
            .map(|(name, (count, size))| (name, count, size))
            .collect();
        type_rows.sort_by(|a, b| b.2.cmp(&a.2));

        let rows: Vec<Vec<String>> = type_rows
            .iter()
            .map(|(name, count, size)| {
                vec![
                    name.clone(),
                    count.to_string(),
                    crate::utils::format_size(*size),
                ]
            })
            .collect();

        report.add_table(&["Type", "Count", "Size"], rows);

        // Top items
        report.add_subsection("Top 20 Largest Items");

        let top_items = result.top_n(20);
        let rows: Vec<Vec<String>> = top_items
            .iter()
            .map(|e| {
                vec![
                    e.path.display().to_string(),
                    e.file_type.display_name().to_string(),
                    e.size_human(),
                ]
            })
            .collect();

        report.add_table(&["Path", "Type", "Size"], rows);
    }

    // Package Manager Caches
    report.add_section("Package Manager Caches");

    let caches = list_package_manager_caches();
    let cache_rows: Vec<Vec<String>> = caches
        .iter()
        .filter(|(_, size, _)| *size > 0)
        .map(|(manager, size, installed)| {
            vec![
                manager.display_name().to_string(),
                crate::utils::format_size(*size),
                if *installed { "Yes" } else { "No" }.to_string(),
            ]
        })
        .collect();

    if !cache_rows.is_empty() {
        report.add_table(&["Manager", "Cache Size", "Installed"], cache_rows);
    } else {
        report.add_paragraph("No significant package manager caches found.");
    }

    // Application Remnants
    report.add_section("Application Remnants");

    match ApplicationScanner::find_remnants() {
        Ok(remnants) if !remnants.is_empty() => {
            let top_remnants: Vec<_> = remnants.into_iter().take(20).collect();
            let rows: Vec<Vec<String>> = top_remnants
                .iter()
                .map(|r| {
                    vec![
                        r.app_name.clone(),
                        r.paths.len().to_string(),
                        r.size_human(),
                    ]
                })
                .collect();

            report.add_table(&["App Name", "Files", "Size"], rows);
        }
        _ => {
            report.add_paragraph("No significant application remnants found.");
        }
    }

    // Recommendations
    report.add_section("Recommendations");

    report.add_paragraph(
        "Based on the analysis, here are recommended cleanup actions (in order of impact):",
    );

    let mut recommendations = Vec::new();

    if let Some(result) = scan_result {
        // Check for large node_modules
        let node_modules_size: u64 = result
            .entries
            .iter()
            .filter(|e| e.file_type == crate::scanner::FileType::NodeModules)
            .map(|e| e.size_bytes)
            .sum();

        if node_modules_size > 1024 * 1024 * 100 {
            // > 100MB
            recommendations.push(format!(
                "1. **Clean node_modules**: {} - Run `macpurge clean node_modules {}`",
                crate::utils::format_size(node_modules_size),
                result.root_path.display()
            ));
        }

        // Check for Rust targets
        let rust_target_size: u64 = result
            .entries
            .iter()
            .filter(|e| e.file_type == crate::scanner::FileType::RustTarget)
            .map(|e| e.size_bytes)
            .sum();

        if rust_target_size > 1024 * 1024 * 500 {
            // > 500MB
            recommendations.push(format!(
                "2. **Clean Rust targets**: {} - Run `macpurge clean rust {}`",
                crate::utils::format_size(rust_target_size),
                result.root_path.display()
            ));
        }

        // Check for Python venvs
        let venv_size: u64 = result
            .entries
            .iter()
            .filter(|e| e.file_type == crate::scanner::FileType::PythonVenv)
            .map(|e| e.size_bytes)
            .sum();

        if venv_size > 1024 * 1024 * 100 {
            // > 100MB
            recommendations.push(format!(
                "3. **Clean Python venvs**: {} - Run `macpurge clean venv {}`",
                crate::utils::format_size(venv_size),
                result.root_path.display()
            ));
        }
    }

    // Package manager cache recommendations
    for (manager, size, installed) in list_package_manager_caches() {
        if installed && size > 1024 * 1024 * 100 {
            // > 100MB
            recommendations.push(format!(
                "4. **Clean {} cache**: {} - Run `macpurge clean {}`",
                manager.display_name(),
                crate::utils::format_size(size),
                manager.display_name().to_lowercase()
            ));
        }
    }

    if recommendations.is_empty() {
        recommendations.push("No significant cleanup opportunities found.".to_string());
    }

    report.add_list(&recommendations);

    // Commands summary
    report.add_section("Quick Commands");

    report.add_code_block(
        "# Scan a directory for dev artifacts
macpurge scan ~/projects --dev

# Clean all node_modules
macpurge clean node_modules ~/projects

# Clean package manager caches
macpurge clean npm
macpurge clean cargo
macpurge clean brew

# Find application remnants
macpurge apps remnants

# Generate this report
macpurge analyze",
        "bash",
    );

    Ok(report.content().to_string())
}

/// Generate a simple summary for quick viewing
pub fn generate_summary(scan_result: &ScanResult) -> String {
    let mut summary = String::new();

    summary.push_str(&format!(
        "Scan Summary for: {}\n",
        scan_result.root_path.display()
    ));
    summary.push_str(&format!("Total cleanable: {}\n", scan_result.total_size_human()));
    summary.push_str(&format!("Items found: {}\n", scan_result.entries.len()));
    summary.push_str(&format!(
        "Scan time: {}ms\n\n",
        scan_result.scan_duration_ms
    ));

    summary.push_str("Top 5 largest:\n");
    for entry in scan_result.top_n(5) {
        summary.push_str(&format!(
            "  {} - {} ({})\n",
            entry.path.display(),
            entry.file_type.display_name(),
            entry.size_human()
        ));
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_report_creation() {
        let mut report = MarkdownReport::new();
        report.add_header("Test Report");
        report.add_section("Test Section");
        report.add_paragraph("Test paragraph");

        let content = report.content();
        assert!(content.contains("# Test Report"));
        assert!(content.contains("## Test Section"));
        assert!(content.contains("Test paragraph"));
    }
}
