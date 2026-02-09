use colored::Colorize;
use std::io::{self, Write};

use crate::cleaners::CleanResult;
use crate::scanner::{DiskInfo, ScanResult, ScannedEntry};

/// Display scan results in an interactive terminal format
pub fn display_scan_results(result: &ScanResult) {
    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .bright_blue()
    );
    println!(
        "  {} {}",
        "Scan Results for:".bold(),
        result.root_path.display().to_string().cyan()
    );
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .bright_blue()
    );
    println!();

    // Summary
    println!(
        "  {} {}",
        "Total cleanable:".bold(),
        result.total_size_human().green().bold()
    );
    println!(
        "  {} {}",
        "Items found:".bold(),
        result.entries.len().to_string().yellow()
    );
    println!(
        "  {} {}ms",
        "Scan time:".bold(),
        result.scan_duration_ms
    );
    println!();

    // By category
    if !result.by_type.is_empty() {
        println!("  {}", "By Category:".bold().underline());
        println!();

        let mut categories: Vec<_> = result.by_type.iter().collect();
        categories.sort_by(|a, b| {
            let size_a: u64 = a.1.iter().map(|e| e.size_bytes).sum();
            let size_b: u64 = b.1.iter().map(|e| e.size_bytes).sum();
            size_b.cmp(&size_a)
        });

        for (category, entries) in categories {
            let total_size: u64 = entries.iter().map(|e| e.size_bytes).sum();
            let size_str = crate::utils::format_size(total_size);
            println!(
                "    {:.<40} {:>6} items  {:>10}",
                category.bright_white(),
                entries.len().to_string().yellow(),
                size_str.green()
            );
        }
        println!();
    }

    // Top items
    println!("  {}", "Top 10 Largest Items:".bold().underline());
    println!();

    let top_items = result.top_n(10);
    for (i, entry) in top_items.iter().enumerate() {
        print_entry(i + 1, entry);
    }

    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .bright_blue()
    );
}

/// Print a single entry in formatted style
fn print_entry(index: usize, entry: &ScannedEntry) {
    let path = entry.path.display().to_string();
    let truncated_path = if path.len() > 60 {
        format!("...{}", &path[path.len() - 57..])
    } else {
        path
    };

    println!(
        "    {:>2}. {} {}",
        index.to_string().bright_black(),
        truncated_path.cyan(),
        format!("[{}]", entry.file_type.display_name()).bright_black()
    );
    println!(
        "        {} {}  {} files",
        "Size:".bright_black(),
        entry.size_human().green().bold(),
        entry.file_count.to_string().yellow()
    );
}

/// Display disk information
pub fn display_disk_info(disks: &[DiskInfo]) {
    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .bright_blue()
    );
    println!("  {}", "Disk Usage".bold());
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .bright_blue()
    );
    println!();

    for disk in disks {
        let usage_bar = create_usage_bar(disk.usage_percent, 30);
        let usage_color = if disk.usage_percent > 90.0 {
            disk.usage_percent.to_string().red()
        } else if disk.usage_percent > 70.0 {
            disk.usage_percent.to_string().yellow()
        } else {
            disk.usage_percent.to_string().green()
        };

        println!("  {} ({})", disk.name.bold(), disk.mount_point.display());
        println!(
            "    {} {:>10} / {:>10}  [{:.1}%]",
            usage_bar,
            disk.used_human().yellow(),
            disk.total_human().bright_white(),
            usage_color
        );
        println!(
            "    {} {}",
            "Available:".bright_black(),
            disk.available_human().green()
        );
        println!();
    }

    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .bright_blue()
    );
}

/// Create a visual usage bar
fn create_usage_bar(percent: f64, width: usize) -> String {
    let filled = ((percent / 100.0) * width as f64) as usize;
    let empty = width.saturating_sub(filled);

    let bar_char = "█";
    let empty_char = "░";

    let bar = format!(
        "{}{}",
        bar_char.repeat(filled),
        empty_char.repeat(empty)
    );

    if percent > 90.0 {
        bar.red().to_string()
    } else if percent > 70.0 {
        bar.yellow().to_string()
    } else {
        bar.green().to_string()
    }
}

/// Display clean results
pub fn display_clean_results(result: &CleanResult, dry_run: bool) {
    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .bright_blue()
    );

    if dry_run {
        println!("  {} {}", "DRY RUN".yellow().bold(), "- No files were deleted".yellow());
    } else {
        println!("  {}", "Clean Results".bold());
    }

    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .bright_blue()
    );
    println!();

    let action = if dry_run { "Would remove" } else { "Removed" };

    println!(
        "  {} {} items",
        format!("{}:", action).bold(),
        result.items_removed.to_string().yellow()
    );
    println!(
        "  {} {}",
        "Space freed:".bold(),
        result.bytes_freed_human().green().bold()
    );

    if !result.errors.is_empty() {
        println!();
        println!("  {} {}", "Errors:".red().bold(), result.errors.len());
        for error in &result.errors {
            println!("    {} {}", "!".red(), error.bright_black());
        }
    }

    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .bright_blue()
    );
}

/// Prompt user for confirmation
pub fn confirm(message: &str) -> bool {
    print!("{} {} ", message.yellow(), "[y/N]:".bright_black());
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

/// Display a warning message
pub fn warn(message: &str) {
    println!("{} {}", "WARNING:".yellow().bold(), message);
}

/// Display an error message
pub fn error(message: &str) {
    println!("{} {}", "ERROR:".red().bold(), message);
}

/// Display a success message
pub fn success(message: &str) {
    println!("{} {}", "SUCCESS:".green().bold(), message);
}

/// Display an info message
pub fn info(message: &str) {
    println!("{} {}", "INFO:".cyan().bold(), message);
}

/// Display application list
pub fn display_apps(apps: &[crate::cleaners::AppInfo]) {
    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .bright_blue()
    );
    println!(
        "  {} ({} apps)",
        "Installed Applications".bold(),
        apps.len()
    );
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .bright_blue()
    );
    println!();

    for app in apps.iter().take(30) {
        let version = app.version.as_deref().unwrap_or("N/A");
        println!(
            "  {:.<50} {:>10}  v{}",
            app.name.bright_white(),
            app.size_human().green(),
            version.bright_black()
        );
    }

    if apps.len() > 30 {
        println!();
        println!(
            "  {} {} more apps...",
            "...and".bright_black(),
            (apps.len() - 30).to_string().yellow()
        );
    }

    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .bright_blue()
    );
}

/// Display application remnants
pub fn display_remnants(remnants: &[crate::cleaners::AppRemnant]) {
    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .bright_blue()
    );
    println!(
        "  {} ({} found)",
        "Application Remnants".bold(),
        remnants.len()
    );
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .bright_blue()
    );
    println!();

    if remnants.is_empty() {
        println!("  {}", "No application remnants found.".bright_black());
    } else {
        let total_size: u64 = remnants.iter().map(|r| r.total_size).sum();
        println!(
            "  {} {}",
            "Total recoverable space:".bold(),
            crate::utils::format_size(total_size).green().bold()
        );
        println!();

        for remnant in remnants.iter().take(20) {
            println!(
                "  {:.<50} {:>10}  ({} files)",
                remnant.app_name.yellow(),
                remnant.size_human().green(),
                remnant.paths.len().to_string().bright_black()
            );
        }

        if remnants.len() > 20 {
            println!();
            println!(
                "  {} {} more remnants...",
                "...and".bright_black(),
                (remnants.len() - 20).to_string().yellow()
            );
        }
    }

    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .bright_blue()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_usage_bar() {
        let bar = create_usage_bar(50.0, 10);
        assert!(!bar.is_empty());
    }
}
