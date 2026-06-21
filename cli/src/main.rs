//! macpurge - CLI tool for cleaning macOS disk space

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

use macpurge::*;

#[derive(Parser)]
#[command(name = "macpurge")]
#[command(author = "Evgeny <github.com/suenot>")]
#[command(version)]
#[command(about = "CLI tool for cleaning macOS disk space", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan project directories from config for dev artifacts
    ScanDev {
        /// Override project directories from config
        #[arg(long)]
        dirs: Option<Vec<PathBuf>>,

        /// Minimum size to report (e.g., "100MB")
        #[arg(long)]
        min_size: Option<String>,

        /// Only scan artifacts older than N days
        #[arg(long)]
        older_than: Option<u32>,

        /// Output format (text, json)
        #[arg(long, short, default_value = "text")]
        output: String,
    },

    /// Initialize or update configuration
    Config {
        /// Add a project directory
        #[arg(long)]
        add_dir: Option<PathBuf>,

        /// Remove a project directory
        #[arg(long)]
        remove_dir: Option<PathBuf>,

        /// Show current configuration
        #[arg(long)]
        show: bool,

        /// Initialize default configuration
        #[arg(long)]
        init: bool,
    },

    /// Scan directory for cleanable items
    Scan {
        /// Path to scan (default: current directory)
        path: Option<PathBuf>,

        /// Scan all user disks
        #[arg(long)]
        all: bool,

        /// Developer mode - scan for all dev artifacts
        #[arg(long, short)]
        dev: bool,

        /// Include hidden directories
        #[arg(long)]
        hidden: bool,

        /// Maximum scan depth
        #[arg(long)]
        max_depth: Option<usize>,

        /// Minimum size to report (e.g., "100MB")
        #[arg(long)]
        min_size: Option<String>,

        /// Output format (text, json)
        #[arg(long, short, default_value = "text")]
        output: String,
    },

    /// Clean specific artifact types
    Clean {
        /// Type of artifacts to clean
        #[command(subcommand)]
        target: CleanTarget,
    },

    /// Generate analysis report for Claude
    Analyze {
        /// Path to analyze
        path: Option<PathBuf>,

        /// Output file (default: stdout)
        #[arg(long, short)]
        output: Option<PathBuf>,
    },

    /// Ask Claude a question about disk usage
    Ask {
        /// Question to ask Claude
        question: String,
    },

    /// Manage applications
    Apps {
        #[command(subcommand)]
        action: AppsAction,
    },

    /// Show disk usage information
    Disks,

    /// Scan package manager caches (npm, cargo, pip, brew, etc.)
    ScanPackages {
        /// Specific package manager to scan (npm, yarn, pnpm, pip, cargo, rustup, go, gem, brew, composer)
        #[arg(long)]
        manager: Option<String>,

        /// Output format (text, json)
        #[arg(long, short, default_value = "text")]
        output: String,
    },

    /// Scan system caches, logs, trash, downloads, and more
    ScanSystem {
        /// Output format (text, json)
        #[arg(long, short, default_value = "text")]
        output: String,
    },

    /// Scan development tools (Xcode, Android Studio, Docker, VMs)
    ScanDevtools {
        /// Specific tool to scan (xcode, android, docker, vm)
        #[arg(long)]
        tool: Option<String>,

        /// Output format (text, json)
        #[arg(long, short, default_value = "text")]
        output: String,
    },

    /// Scan AI/ML model artifacts (Ollama, LM Studio, Hugging Face, ComfyUI)
    ScanAi {
        /// Specific tool to scan (ollama, lmstudio, huggingface, comfyui)
        #[arg(long)]
        tool: Option<String>,

        /// Output format (text, json)
        #[arg(long, short, default_value = "text")]
        output: String,
    },

    /// Manage cleanup recipes
    Recipe {
        #[command(subcommand)]
        action: RecipeCommand,
    },

    /// Manage scheduled cleanup
    Schedule {
        #[command(subcommand)]
        action: ScheduleCommand,
    },
}

#[derive(Subcommand)]
enum CleanTarget {
    /// Clean node_modules directories
    NodeModules {
        /// Path to scan
        path: PathBuf,

        /// Dry run - show what would be deleted
        #[arg(long, short = 'n')]
        dry_run: bool,

        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// Verbose output
        #[arg(long, short)]
        verbose: bool,
    },

    /// Clean Python virtual environments
    Venv {
        /// Path to scan
        path: PathBuf,

        /// Dry run - show what would be deleted
        #[arg(long, short = 'n')]
        dry_run: bool,

        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// Verbose output
        #[arg(long, short)]
        verbose: bool,
    },

    /// Clean Rust target directories
    Rust {
        /// Path to scan
        path: PathBuf,

        /// Dry run - show what would be deleted
        #[arg(long, short = 'n')]
        dry_run: bool,

        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// Verbose output
        #[arg(long, short)]
        verbose: bool,
    },

    /// Clean Cargo global cache
    Cargo {
        /// Dry run - show what would be deleted
        #[arg(long, short = 'n')]
        dry_run: bool,

        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// Verbose output
        #[arg(long, short)]
        verbose: bool,
    },

    /// Clean npm global cache
    Npm {
        /// Dry run - show what would be deleted
        #[arg(long, short = 'n')]
        dry_run: bool,

        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// Verbose output
        #[arg(long, short)]
        verbose: bool,
    },

    /// Clean Homebrew cache
    Brew {
        /// Dry run - show what would be deleted
        #[arg(long, short = 'n')]
        dry_run: bool,

        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// Verbose output
        #[arg(long, short)]
        verbose: bool,
    },

    /// Clean pip cache
    Pip {
        /// Dry run - show what would be deleted
        #[arg(long, short = 'n')]
        dry_run: bool,

        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// Verbose output
        #[arg(long, short)]
        verbose: bool,
    },

    /// Clean all dev artifacts in a directory
    All {
        /// Path to scan
        path: PathBuf,

        /// Dry run - show what would be deleted
        #[arg(long, short = 'n')]
        dry_run: bool,

        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// Verbose output
        #[arg(long, short)]
        verbose: bool,
    },

    /// Clean user caches
    Cache {
        /// Dry run - show what would be deleted
        #[arg(long, short = 'n')]
        dry_run: bool,

        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// Verbose output
        #[arg(long, short)]
        verbose: bool,
    },

    /// Clean system logs
    Logs {
        /// Dry run - show what would be deleted
        #[arg(long, short = 'n')]
        dry_run: bool,

        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// Verbose output
        #[arg(long, short)]
        verbose: bool,
    },

    /// Clean iOS device backups
    IosBackups {
        /// Dry run - show what would be deleted
        #[arg(long, short = 'n')]
        dry_run: bool,

        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// Verbose output
        #[arg(long, short)]
        verbose: bool,
    },

    /// Empty the Trash
    Trash {
        /// Dry run - show what would be deleted
        #[arg(long, short = 'n')]
        dry_run: bool,

        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// Verbose output
        #[arg(long, short)]
        verbose: bool,
    },

    /// Clean old downloads
    Downloads {
        /// Only delete files older than N days (default: 30)
        #[arg(long, default_value = "30")]
        older_than: u64,

        /// Dry run - show what would be deleted
        #[arg(long, short = 'n')]
        dry_run: bool,

        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// Verbose output
        #[arg(long, short)]
        verbose: bool,
    },

    /// Clean mail caches
    Mail {
        /// Dry run - show what would be deleted
        #[arg(long, short = 'n')]
        dry_run: bool,

        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// Verbose output
        #[arg(long, short)]
        verbose: bool,
    },

    /// Clean Xcode DerivedData and caches
    Xcode {
        /// What to clean: derived-data, archives, simulators, device-support, cache, all
        #[arg(long, default_value = "derived-data")]
        what: String,

        /// Dry run - show what would be deleted
        #[arg(long, short = 'n')]
        dry_run: bool,

        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// Verbose output
        #[arg(long, short)]
        verbose: bool,
    },

    /// Clean Docker resources
    Docker {
        /// Dry run - show what would be deleted
        #[arg(long, short = 'n')]
        dry_run: bool,

        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// Verbose output
        #[arg(long, short)]
        verbose: bool,
    },

    /// Clean Gradle cache
    Gradle {
        /// Dry run - show what would be deleted
        #[arg(long, short = 'n')]
        dry_run: bool,

        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// Verbose output
        #[arg(long, short)]
        verbose: bool,
    },
}

#[derive(Subcommand)]
enum AppsAction {
    /// List installed applications
    List,

    /// Find remnants of uninstalled applications
    Remnants,

    /// Uninstall an application completely
    Uninstall {
        /// Application name
        name: String,

        /// Dry run - show what would be deleted
        #[arg(long, short = 'n')]
        dry_run: bool,

        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// Verbose output
        #[arg(long, short)]
        verbose: bool,
    },
}

#[derive(Subcommand)]
enum RecipeCommand {
    /// List all available recipes
    List,

    /// Run a recipe
    Run {
        /// Recipe ID
        id: String,

        /// Dry run - show what would be cleaned
        #[arg(long, short = 'n')]
        dry_run: bool,

        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum ScheduleCommand {
    /// Enable scheduled cleanup
    Enable {
        /// Schedule: daily, weekly, monthly, or cron expression (e.g. "0 3 * * *")
        #[arg(default_value = "daily")]
        cron: String,
    },

    /// Disable scheduled cleanup
    Disable,

    /// Show schedule status
    Status,

    /// Run scheduled tasks now
    RunNow,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::ScanDev {
            dirs,
            min_size,
            older_than,
            output,
        } => {
            cmd_scan_dev(dirs, min_size, older_than, output)?;
        }

        Commands::Config {
            add_dir,
            remove_dir,
            show,
            init,
        } => {
            cmd_config(add_dir, remove_dir, show, init)?;
        }

        Commands::Scan {
            path,
            all,
            dev,
            hidden,
            max_depth,
            min_size,
            output,
        } => {
            cmd_scan(path, all, dev, hidden, max_depth, min_size, output)?;
        }

        Commands::Clean { target } => {
            cmd_clean(target)?;
        }

        Commands::Analyze { path, output } => {
            cmd_analyze(path, output)?;
        }

        Commands::Ask { question } => {
            cmd_ask(&question)?;
        }

        Commands::Apps { action } => {
            cmd_apps(action)?;
        }

        Commands::Disks => {
            cmd_disks()?;
        }

        Commands::ScanPackages { manager, output } => {
            cmd_scan_packages(manager, output)?;
        }

        Commands::ScanSystem { output } => {
            cmd_scan_system(output)?;
        }

        Commands::ScanDevtools { tool, output } => {
            cmd_scan_devtools(tool, output)?;
        }

        Commands::ScanAi { tool, output } => {
            cmd_scan_ai(tool, output)?;
        }

        Commands::Recipe { action } => {
            cmd_recipe(action)?;
        }

        Commands::Schedule { action } => {
            cmd_schedule(action)?;
        }
    }

    Ok(())
}

fn cmd_scan(
    path: Option<PathBuf>,
    all: bool,
    dev: bool,
    hidden: bool,
    max_depth: Option<usize>,
    min_size: Option<String>,
    output: String,
) -> Result<()> {
    let min_size_bytes = min_size.and_then(|s| utils::parse_size(&s));

    // Honor the global "never scan" exclude list.
    let excluded = macpurge::get_excluded_dirs(&macpurge::load_config().unwrap_or_default());

    let options = ScanOptions {
        dev_mode: dev,
        include_hidden: hidden,
        max_depth,
        min_size: min_size_bytes,
        show_progress: output == "text",
        excluded,
    };

    let scanner = DirectoryScanner::new(options);

    let paths: Vec<PathBuf> = if all {
        get_user_disks()?
            .into_iter()
            .map(|d| d.mount_point)
            .collect()
    } else {
        vec![path.unwrap_or_else(|| std::env::current_dir().unwrap_or_default())]
    };

    for scan_path in &paths {
        if output == "text" {
            println!(
                "\n{} {}",
                "Scanning:".cyan().bold(),
                scan_path.display()
            );
        }

        match scanner.scan(scan_path) {
            Ok(result) => {
                if output == "json" {
                    println!("{}", generate_json_report(result)?);
                } else {
                    display_scan_results(&result);
                }
            }
            Err(e) => {
                error(&format!("Failed to scan {}: {}", scan_path.display(), e));
            }
        }
    }

    Ok(())
}

fn cmd_clean(target: CleanTarget) -> Result<()> {
    // Honor the configured deletion mode (move to Trash vs. permanent delete).
    // Default to the safe, reversible behavior if the config can't be read.
    let use_trash = macpurge::load_config()
        .map(|c| c.cleanup.use_trash)
        .unwrap_or(true);

    match target {
        CleanTarget::NodeModules {
            path,
            dry_run,
            yes,
            verbose,
        } => {
            let options = CleanOptions {
                dry_run,
                force: yes,
                verbose,
                use_trash,
                ..Default::default()
            };

            let estimate = DevArtifactsCleaner::node_modules(path.clone()).estimate()?;

            if estimate == 0 {
                info("No node_modules found.");
                return Ok(());
            }

            println!(
                "Found node_modules totaling {}",
                utils::format_size(estimate).green().bold()
            );

            if !yes && !dry_run && !confirm("Proceed with cleanup?") {
                println!("Aborted.");
                return Ok(());
            }

            let result = clean_node_modules(&path, &options)?;
            display_clean_results(&result, dry_run);
        }

        CleanTarget::Venv {
            path,
            dry_run,
            yes,
            verbose,
        } => {
            let options = CleanOptions {
                dry_run,
                force: yes,
                verbose,
                use_trash,
                ..Default::default()
            };

            let estimate = DevArtifactsCleaner::python_venv(path.clone()).estimate()?;

            if estimate == 0 {
                info("No Python virtual environments found.");
                return Ok(());
            }

            println!(
                "Found Python venvs totaling {}",
                utils::format_size(estimate).green().bold()
            );

            if !yes && !dry_run && !confirm("Proceed with cleanup?") {
                println!("Aborted.");
                return Ok(());
            }

            let result = clean_python_venv(&path, &options)?;
            display_clean_results(&result, dry_run);
        }

        CleanTarget::Rust {
            path,
            dry_run,
            yes,
            verbose,
        } => {
            let options = CleanOptions {
                dry_run,
                force: yes,
                verbose,
                use_trash,
                ..Default::default()
            };

            let estimate = DevArtifactsCleaner::rust_target(path.clone()).estimate()?;

            if estimate == 0 {
                info("No Rust target directories found.");
                return Ok(());
            }

            println!(
                "Found Rust targets totaling {}",
                utils::format_size(estimate).green().bold()
            );

            if !yes && !dry_run && !confirm("Proceed with cleanup?") {
                println!("Aborted.");
                return Ok(());
            }

            let result = clean_rust_target(&path, &options)?;
            display_clean_results(&result, dry_run);
        }

        CleanTarget::Cargo {
            dry_run,
            yes,
            verbose,
        } => {
            let options = CleanOptions {
                dry_run,
                force: yes,
                verbose,
                use_trash,
                ..Default::default()
            };

            let estimate = PackageManagerCleaner::cargo().estimate()?;

            if estimate == 0 {
                info("No Cargo cache found.");
                return Ok(());
            }

            println!(
                "Cargo cache size: {}",
                utils::format_size(estimate).green().bold()
            );

            if !yes && !dry_run && !confirm("Proceed with cleanup?") {
                println!("Aborted.");
                return Ok(());
            }

            let result = clean_cargo_cache(&options)?;
            display_clean_results(&result, dry_run);
        }

        CleanTarget::Npm {
            dry_run,
            yes,
            verbose,
        } => {
            let options = CleanOptions {
                dry_run,
                force: yes,
                verbose,
                use_trash,
                ..Default::default()
            };

            let estimate = PackageManagerCleaner::npm().estimate()?;

            if estimate == 0 {
                info("No npm cache found.");
                return Ok(());
            }

            println!(
                "npm cache size: {}",
                utils::format_size(estimate).green().bold()
            );

            if !yes && !dry_run && !confirm("Proceed with cleanup?") {
                println!("Aborted.");
                return Ok(());
            }

            let result = clean_npm_cache(&options)?;
            display_clean_results(&result, dry_run);
        }

        CleanTarget::Brew {
            dry_run,
            yes,
            verbose,
        } => {
            let options = CleanOptions {
                dry_run,
                force: yes,
                verbose,
                use_trash,
                ..Default::default()
            };

            let estimate = PackageManagerCleaner::homebrew().estimate()?;

            if estimate == 0 {
                info("No Homebrew cache found.");
                return Ok(());
            }

            println!(
                "Homebrew cache size: {}",
                utils::format_size(estimate).green().bold()
            );

            if !yes && !dry_run && !confirm("Proceed with cleanup?") {
                println!("Aborted.");
                return Ok(());
            }

            let result = clean_homebrew_cache(&options)?;
            display_clean_results(&result, dry_run);
        }

        CleanTarget::Pip {
            dry_run,
            yes,
            verbose,
        } => {
            let options = CleanOptions {
                dry_run,
                force: yes,
                verbose,
                use_trash,
                ..Default::default()
            };

            let estimate = PackageManagerCleaner::pip().estimate()?;

            if estimate == 0 {
                info("No pip cache found.");
                return Ok(());
            }

            println!(
                "pip cache size: {}",
                utils::format_size(estimate).green().bold()
            );

            if !yes && !dry_run && !confirm("Proceed with cleanup?") {
                println!("Aborted.");
                return Ok(());
            }

            let result = clean_pip_cache(&options)?;
            display_clean_results(&result, dry_run);
        }

        CleanTarget::All {
            path,
            dry_run,
            yes,
            verbose,
        } => {
            let options = CleanOptions {
                dry_run,
                force: yes,
                verbose,
                use_trash,
                ..Default::default()
            };

            let estimate = DevArtifactsCleaner::all(path.clone()).estimate()?;

            if estimate == 0 {
                info("No dev artifacts found.");
                return Ok(());
            }

            println!(
                "Found dev artifacts totaling {}",
                utils::format_size(estimate).green().bold()
            );

            if !yes && !dry_run && !confirm("Proceed with cleanup?") {
                println!("Aborted.");
                return Ok(());
            }

            let result = clean_all_dev_artifacts(&path, &options)?;
            display_clean_results(&result, dry_run);
        }

        CleanTarget::Cache {
            dry_run,
            yes,
            verbose,
        } => {
            let options = CleanOptions {
                dry_run,
                force: yes,
                verbose,
                use_trash,
                ..Default::default()
            };

            let cleaner = SystemCacheCleaner::user_cache();
            let estimate = cleaner.estimate()?;

            if estimate == 0 {
                info("No user cache found.");
                return Ok(());
            }

            warn("Cleaning user caches may affect application performance.");
            println!(
                "User cache size: {}",
                utils::format_size(estimate).green().bold()
            );

            if !yes && !dry_run && !confirm("Proceed with cleanup?") {
                println!("Aborted.");
                return Ok(());
            }

            let result = clean_user_cache(&options)?;
            display_clean_results(&result, dry_run);
        }

        CleanTarget::Logs { dry_run, yes, verbose } => {
            clean_target_simple("system logs", SystemCacheCleaner::system_logs(), dry_run, yes, verbose, use_trash)?;
        }

        CleanTarget::IosBackups { dry_run, yes, verbose } => {
            warn("Deleting iOS backups is irreversible. Make sure you don't need them.");
            clean_target_simple("iOS backups", SystemCacheCleaner::ios_backups(), dry_run, yes, verbose, use_trash)?;
        }

        CleanTarget::Trash { dry_run, yes, verbose } => {
            clean_target_simple("Trash", SystemCacheCleaner::trash(), dry_run, yes, verbose, use_trash)?;
        }

        CleanTarget::Downloads { older_than, dry_run, yes, verbose } => {
            let options = CleanOptions {
                dry_run,
                force: yes,
                verbose,
                min_age_days: Some(older_than),
                use_trash,
            };

            let cleaner = SystemCacheCleaner::downloads_old();
            let estimate = cleaner.estimate()?;

            if estimate == 0 {
                info(&format!("No downloads older than {} days found.", older_than));
                return Ok(());
            }

            println!(
                "Old downloads (>{} days): {}",
                older_than,
                utils::format_size(estimate).green().bold()
            );

            if !yes && !dry_run && !confirm("Proceed with cleanup?") {
                println!("Aborted.");
                return Ok(());
            }

            let result = cleaner.clean(&options)?;
            display_clean_results(&result, dry_run);
        }

        CleanTarget::Mail { dry_run, yes, verbose } => {
            clean_target_simple("mail caches", SystemCacheCleaner::mail_caches(), dry_run, yes, verbose, use_trash)?;
        }

        CleanTarget::Xcode { what, dry_run, yes, verbose } => {
            let cleaner = match what.as_str() {
                "derived-data" => DevToolsCleaner::xcode_derived_data(),
                "archives" => DevToolsCleaner::xcode_archives(),
                "simulators" => DevToolsCleaner::xcode_simulators(),
                "device-support" => DevToolsCleaner::xcode_device_support(),
                "cache" => DevToolsCleaner::xcode_cache(),
                "all" => {
                    // Clean all Xcode caches
                    let options = CleanOptions { dry_run, force: yes, verbose, use_trash, ..Default::default() };
                    let mut total = CleanResult::new();
                    for c in [
                        DevToolsCleaner::xcode_derived_data(),
                        DevToolsCleaner::xcode_archives(),
                        DevToolsCleaner::xcode_simulators(),
                        DevToolsCleaner::xcode_device_support(),
                        DevToolsCleaner::xcode_cache(),
                    ] {
                        if let Ok(r) = c.clean(&options) {
                            total.merge(r);
                        }
                    }
                    display_clean_results(&total, dry_run);
                    return Ok(());
                }
                _ => {
                    error(&format!("Unknown Xcode target: {}", what));
                    println!("Available: derived-data, archives, simulators, device-support, cache, all");
                    return Ok(());
                }
            };
            clean_target_simple(&format!("Xcode {}", what), cleaner, dry_run, yes, verbose, use_trash)?;
        }

        CleanTarget::Docker { dry_run, yes, verbose } => {
            let options = CleanOptions { dry_run, force: yes, verbose, use_trash, ..Default::default() };
            if !yes && !dry_run && !confirm("Run docker system prune?") {
                println!("Aborted.");
                return Ok(());
            }
            let result = clean_docker_dangling(&options)?;
            display_clean_results(&result, dry_run);
        }

        CleanTarget::Gradle { dry_run, yes, verbose } => {
            clean_target_simple("Gradle cache", DevToolsCleaner::gradle_cache(), dry_run, yes, verbose, use_trash)?;
        }
    }

    Ok(())
}

/// Helper for simple clean targets that follow the estimate → confirm → clean pattern
fn clean_target_simple(
    label: &str,
    cleaner: impl Cleaner,
    dry_run: bool,
    yes: bool,
    verbose: bool,
    use_trash: bool,
) -> Result<()> {
    let options = CleanOptions {
        dry_run,
        force: yes,
        verbose,
        use_trash,
        ..Default::default()
    };

    let estimate = cleaner.estimate()?;

    if estimate == 0 {
        info(&format!("No {} found.", label));
        return Ok(());
    }

    println!(
        "{} size: {}",
        label,
        utils::format_size(estimate).green().bold()
    );

    if !yes && !dry_run && !confirm("Proceed with cleanup?") {
        println!("Aborted.");
        return Ok(());
    }

    let result = cleaner.clean(&options)?;
    display_clean_results(&result, dry_run);
    Ok(())
}

fn cmd_analyze(path: Option<PathBuf>, output: Option<PathBuf>) -> Result<()> {
    info("Generating analysis report...");

    let report = ClaudeIntegration::generate_analysis_report(path.as_deref())?;

    if let Some(output_path) = output {
        std::fs::write(&output_path, &report)?;
        success(&format!("Report saved to {}", output_path.display()));
    } else {
        println!("{}", report);
    }

    Ok(())
}

fn cmd_ask(question: &str) -> Result<()> {
    if !ClaudeIntegration::is_available() {
        error("Claude CLI not found. Please install it from https://claude.ai/download");
        return Ok(());
    }

    info("Asking Claude...");

    match ClaudeIntegration::ask(question) {
        Ok(response) => {
            println!("\n{}\n", response);
        }
        Err(e) => {
            error(&format!("Failed to get response: {}", e));
        }
    }

    Ok(())
}

fn cmd_apps(action: AppsAction) -> Result<()> {
    // Honor the configured deletion mode (move to Trash vs. permanent delete).
    let use_trash = macpurge::load_config()
        .map(|c| c.cleanup.use_trash)
        .unwrap_or(true);

    match action {
        AppsAction::List => {
            info("Scanning installed applications...");
            let apps = ApplicationScanner::list_apps()?;
            display_apps(&apps);
        }

        AppsAction::Remnants => {
            info("Scanning for application remnants...");
            let remnants = ApplicationScanner::find_remnants()?;
            display_remnants(&remnants);
        }

        AppsAction::Uninstall {
            name,
            dry_run,
            yes,
            verbose,
        } => {
            let options = CleanOptions {
                dry_run,
                force: yes,
                verbose,
                use_trash,
                ..Default::default()
            };

            info(&format!("Finding files for {}...", name));

            let files = ApplicationScanner::find_app_files(&name)?;

            if files.is_empty() {
                error(&format!("No files found for application: {}", name));
                return Ok(());
            }

            println!(
                "Found {} files/directories related to {}",
                files.len().to_string().yellow(),
                name.cyan()
            );

            for file in &files {
                println!("  - {}", file.display());
            }

            if !yes && !dry_run && !confirm(&format!("Uninstall {} completely?", name)) {
                println!("Aborted.");
                return Ok(());
            }

            // Delete the exact list found and confirmed above. Passing the
            // captured `files` (rather than re-scanning inside uninstall_app)
            // closes the TOCTOU window and guarantees we remove precisely what
            // the user approved (GitLab #13).
            let result = ApplicationScanner::uninstall_app(&files, &options)?;
            display_clean_results(&result, dry_run);
        }
    }

    Ok(())
}

fn cmd_disks() -> Result<()> {
    let disks = get_all_disks()?;
    display_disk_info(&disks);
    Ok(())
}

fn cmd_scan_dev(
    dirs: Option<Vec<PathBuf>>,
    min_size: Option<String>,
    older_than: Option<u32>,
    output: String,
) -> Result<()> {
    let config = macpurge::load_config()?;

    let project_dirs: Vec<PathBuf> = if let Some(override_dirs) = dirs {
        override_dirs.into_iter().map(|p| macpurge::expand_path(&p)).collect()
    } else {
        macpurge::get_project_dirs(&config)
    };

    if project_dirs.is_empty() {
        warn("No project directories configured.");
        println!("Add directories with: macpurge config --add-dir ~/projects");
        println!("Or specify directly: macpurge scan-dev --dirs ~/projects ~/work");
        return Ok(());
    }

    let min_size_bytes = min_size.and_then(|s| utils::parse_size(&s));

    let options = ScanOptions {
        dev_mode: true,
        include_hidden: config.scan.include_hidden,
        max_depth: Some(config.scan.max_depth),
        min_size: min_size_bytes,
        show_progress: output == "text",
        excluded: macpurge::get_excluded_dirs(&config),
    };

    let scanner = DirectoryScanner::new(options);

    if output == "text" {
        println!(
            "\n{} Scanning {} project directories for dev artifacts...\n",
            "🔍".cyan(),
            project_dirs.len().to_string().green().bold()
        );

        for dir in &project_dirs {
            println!("  • {}", dir.display());
        }
        println!();
    }

    let mut total_size: u64 = 0;
    let mut total_entries: usize = 0;

    for scan_path in &project_dirs {
        if !scan_path.exists() {
            if output == "text" {
                warn(&format!("Directory not found: {}", scan_path.display()));
            }
            continue;
        }

        if output == "text" {
            println!("{} {}", "Scanning:".cyan(), scan_path.display());
        }

        match scanner.scan(scan_path) {
            Ok(result) => {
                // Filter by age if specified
                let filtered_result = if let Some(days) = older_than {
                    filter_by_age(&result, days)
                } else {
                    result
                };

                if output == "json" {
                    println!("{}", generate_json_report(filtered_result)?);
                } else {
                    total_size += filtered_result.total_size;
                    total_entries += filtered_result.entries.len();
                    display_scan_results(&filtered_result);
                }
            }
            Err(e) => {
                error(&format!("Failed to scan {}: {}", scan_path.display(), e));
            }
        }
    }

    if output == "text" && project_dirs.len() > 1 {
        println!("\n{}", "═".repeat(60).dimmed());
        println!(
            "{} Found {} artifacts totaling {}",
            "📊".cyan(),
            total_entries.to_string().yellow().bold(),
            utils::format_size(total_size).green().bold()
        );
    }

    Ok(())
}

fn filter_by_age(result: &ScanResult, days: u32) -> ScanResult {
    use std::time::{SystemTime, UNIX_EPOCH};

    let cutoff = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        - (days as u64 * 24 * 60 * 60);

    let filtered_entries: Vec<ScannedEntry> = result
        .entries
        .iter()
        .filter(|e| {
            e.last_modified
                .map(|ts| (ts as u64) < cutoff)
                .unwrap_or(false)
        })
        .cloned()
        .collect();

    let total_size: u64 = filtered_entries.iter().map(|e| e.size_bytes).sum();

    ScanResult {
        root_path: result.root_path.clone(),
        entries: filtered_entries.clone(),
        total_size,
        total_files: filtered_entries.len() as u64,
        scan_duration_ms: result.scan_duration_ms,
        by_type: result.by_type.clone(),
    }
}

fn cmd_scan_packages(manager: Option<String>, output: String) -> Result<()> {
    let scanner = PackageScanner::new();

    if output == "text" {
        println!(
            "\n{} Scanning package manager caches...\n",
            "📦".cyan()
        );
    }

    let result = if let Some(mgr_name) = manager {
        // Scan specific manager
        let mgr = match mgr_name.to_lowercase().as_str() {
            "npm" => PkgManagerType::Npm,
            "yarn" => PkgManagerType::Yarn,
            "pnpm" => PkgManagerType::Pnpm,
            "pip" => PkgManagerType::Pip,
            "cargo" => PkgManagerType::Cargo,
            "rustup" => PkgManagerType::Rustup,
            "go" => PkgManagerType::Go,
            "gem" => PkgManagerType::Gem,
            "brew" => PkgManagerType::Brew,
            "composer" => PkgManagerType::Composer,
            _ => {
                error(&format!("Unknown package manager: {}", mgr_name));
                println!("Available: npm, yarn, pnpm, pip, cargo, rustup, go, gem, brew, composer");
                return Ok(());
            }
        };

        let info = scanner.scan_manager(mgr)?;
        PackageScanResult {
            managers: vec![info.clone()],
            total_cache_size: info.total_cache_size,
            scan_duration_ms: 0,
        }
    } else {
        // Scan all managers
        scanner.scan_all()?
    };

    if output == "json" {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        display_package_scan_results(&result);
    }

    Ok(())
}

fn display_package_scan_results(result: &PackageScanResult) {
    for info in &result.managers {
        if !info.is_installed && info.cache_paths.is_empty() {
            continue;
        }

        let status = if info.is_installed {
            "✓".green()
        } else {
            "✗".red()
        };

        println!(
            "{} {} {}",
            status,
            info.manager.name().cyan().bold(),
            if info.is_installed { "" } else { "(not installed)" }
        );

        if info.cache_paths.is_empty() {
            println!("    No caches found\n");
            continue;
        }

        for cache in &info.cache_paths {
            let safe_indicator = if cache.safe_to_clean {
                "🗑️ ".to_string()
            } else {
                "⚠️ ".to_string()
            };

            println!(
                "    {} {} {}",
                safe_indicator,
                cache.size_human().yellow(),
                cache.path.display()
            );
            println!("       {}", cache.description.dimmed());
        }

        if !info.global_packages.is_empty() {
            println!("    {} global packages installed", info.global_packages.len());
        }

        println!(
            "    {} Total: {}\n",
            "→".dimmed(),
            info.total_cache_size_human().green().bold()
        );
    }

    println!("{}", "═".repeat(60).dimmed());
    println!(
        "{} Total package cache: {}",
        "📊".cyan(),
        result.total_cache_size_human().green().bold()
    );
    println!(
        "   Scan completed in {}ms\n",
        result.scan_duration_ms.to_string().dimmed()
    );
}

fn cmd_scan_system(output: String) -> Result<()> {
    let scanner = SystemScanner::new();

    if output == "text" {
        println!("\n{} Scanning system caches and cleanable data...\n", "🔧".cyan());
    }

    let result = scanner.scan_all()?;

    if output == "json" {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        for cat in &result.categories {
            println!(
                "{} {} — {}",
                "●".green(),
                cat.name.cyan().bold(),
                cat.size_human().yellow()
            );
            println!("  {}", cat.description.dimmed());

            for path in &cat.paths {
                println!(
                    "    {} {}",
                    crate::utils::format_size(path.size_bytes).yellow(),
                    path.description
                );
            }
            println!();
        }

        println!("{}", "═".repeat(60).dimmed());
        println!(
            "{} Total cleanable: {}",
            "📊".cyan(),
            result.total_size_human().green().bold()
        );
        println!(
            "   Scan completed in {}ms\n",
            result.scan_duration_ms.to_string().dimmed()
        );
    }

    Ok(())
}

fn cmd_scan_devtools(tool: Option<String>, output: String) -> Result<()> {
    let scanner = DevToolScanner::new();

    if output == "text" {
        println!("\n{} Scanning development tools...\n", "🛠️".cyan());
    }

    let result = if let Some(tool_name) = tool {
        let tool_type = match tool_name.to_lowercase().as_str() {
            "xcode" => DevToolType::Xcode,
            "android" => DevToolType::AndroidStudio,
            "docker" => DevToolType::Docker,
            "vm" | "vms" => DevToolType::VirtualMachines,
            _ => {
                error(&format!("Unknown tool: {}", tool_name));
                println!("Available: xcode, android, docker, vm");
                return Ok(());
            }
        };
        let info = scanner.scan_tool(tool_type)?;
        DevToolScanResult {
            total_size: info.total_size,
            tools: vec![info],
            scan_duration_ms: 0,
        }
    } else {
        scanner.scan_all()?
    };

    if output == "json" {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        for tool in &result.tools {
            let status = if tool.is_installed { "✓".green() } else { "✗".red() };
            println!(
                "{} {} — {}",
                status,
                tool.tool.name().cyan().bold(),
                tool.total_size_human().yellow()
            );

            for cache in &tool.cache_paths {
                let safe = if cache.safe_to_clean { "🗑️ " } else { "⚠️ " };
                println!(
                    "    {} {} {}",
                    safe,
                    cache.size_human().yellow(),
                    cache.description
                );
                println!("       {}", cache.path.display().to_string().dimmed());
            }
            println!();
        }

        println!("{}", "═".repeat(60).dimmed());
        println!(
            "{} Total dev tools data: {}",
            "📊".cyan(),
            result.total_size_human().green().bold()
        );
    }

    Ok(())
}

fn cmd_scan_ai(tool: Option<String>, output: String) -> Result<()> {
    let scanner = AiModelScanner::new();

    if output == "text" {
        println!("\n{} Scanning AI/ML model artifacts...\n", "🧠".cyan());
    }

    let result = if let Some(tool_name) = tool {
        let tool_type = match tool_name.to_lowercase().as_str() {
            "ollama" => AiToolType::Ollama,
            "lmstudio" | "lm-studio" | "lm_studio" => AiToolType::LmStudio,
            "huggingface" | "hf" => AiToolType::HuggingFace,
            "comfyui" | "comfy" => AiToolType::ComfyUi,
            _ => {
                error(&format!("Unknown tool: {}", tool_name));
                println!("Available: ollama, lmstudio, huggingface, comfyui");
                return Ok(());
            }
        };
        let info = scanner.scan_tool(tool_type)?;
        AiModelScanResult {
            total_size: info.total_size,
            tools: vec![info],
            scan_duration_ms: 0,
        }
    } else {
        scanner.scan_all()?
    };

    if output == "json" {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        for tool in &result.tools {
            let status = if tool.is_installed { "✓".green() } else { "✗".red() };
            println!(
                "{} {} — {}",
                status,
                tool.tool.name().cyan().bold(),
                tool.total_size_human().yellow()
            );

            for model in &tool.models {
                let safe = if model.safe_to_clean { "🗑️ " } else { "⚠️ " };
                println!(
                    "    {} {} {} {}",
                    safe,
                    model.size_human().yellow(),
                    format!("[{}]", model.category).dimmed(),
                    model.description
                );
                println!("       {}", model.path.display().to_string().dimmed());
            }
            println!();
        }

        println!("{}", "═".repeat(60).dimmed());
        println!(
            "{} Total AI model data: {}",
            "📊".cyan(),
            result.total_size_human().green().bold()
        );
    }

    Ok(())
}

fn cmd_recipe(action: RecipeCommand) -> Result<()> {
    match action {
        RecipeCommand::List => {
            let recipes = get_builtin_recipes();
            println!("\n{} Available cleanup recipes:\n", "📋".cyan());

            for recipe in &recipes {
                println!(
                    "  {} {} — {}",
                    recipe.id.green().bold(),
                    format!("(~{})", recipe.estimated_savings).dimmed(),
                    recipe.name
                );
                println!("     {}", recipe.description.dimmed());
                println!(
                    "     Steps: {}",
                    recipe.steps.iter().map(|s| s.description.as_str()).collect::<Vec<_>>().join(" → ")
                );
                println!();
            }

            println!("Run with: macpurge recipe run <id>");
            println!("Dry run:  macpurge recipe run <id> --dry-run\n");
        }

        RecipeCommand::Run { id, dry_run, yes } => {
            let use_trash = macpurge::load_config()
                .map(|c| c.cleanup.use_trash)
                .unwrap_or(true);
            let options = CleanOptions {
                dry_run,
                force: yes,
                verbose: true,
                use_trash,
                ..Default::default()
            };

            let recipes = get_builtin_recipes();
            let recipe = recipes.iter().find(|r| r.id == id);

            if recipe.is_none() {
                error(&format!("Recipe not found: {}", id));
                println!("Use 'macpurge recipe list' to see available recipes.");
                return Ok(());
            }

            let recipe = recipe.unwrap();
            println!(
                "\n{} Running recipe: {}\n",
                "🚀".cyan(),
                recipe.name.green().bold()
            );

            if !yes && !dry_run && !confirm("Proceed?") {
                println!("Aborted.");
                return Ok(());
            }

            let result = run_recipe(&id, &options)?;

            println!("\n{}", "═".repeat(60).dimmed());
            println!(
                "{} Recipe completed: {}/{} steps",
                "📊".cyan(),
                result.steps_completed.to_string().green(),
                result.total_steps
            );

            for step in &result.step_results {
                let status = if step.success { "✓".green() } else { "✗".red() };
                println!(
                    "  {} {} — freed {}",
                    status,
                    step.description,
                    crate::utils::format_size(step.result.bytes_freed).yellow()
                );
            }

            println!(
                "\n  Total freed: {}\n",
                crate::utils::format_size(result.total_result.bytes_freed).green().bold()
            );
        }
    }

    Ok(())
}

fn cmd_schedule(action: ScheduleCommand) -> Result<()> {
    match action {
        ScheduleCommand::Enable { cron } => {
            let tasks = ScheduleTasks {
                clean_dev: true,
                clean_packages: true,
                clean_docker: true,
                clean_system: true,
                clean_browsers: true,
                empty_trash: true,
            };

            enable_schedule(&cron, &tasks)?;
            success(&format!("Schedule enabled: {}", cron));
            println!("LaunchAgent installed at: {}", get_launch_agent_path()?.display());
        }

        ScheduleCommand::Disable => {
            disable_schedule()?;
            success("Schedule disabled and LaunchAgent removed.");
        }

        ScheduleCommand::Status => {
            let status = get_schedule_status()?;

            println!("\n{} Schedule Status:\n", "⏰".cyan());
            println!("  Enabled:      {}", if status.enabled { "Yes".green() } else { "No".red() });
            println!("  Schedule:     {}", status.cron);
            println!("  Plist:        {}", status.plist_path);
            println!("  Plist exists: {}", status.plist_exists);
            println!("  Loaded:       {}", status.is_loaded);

            println!("\n  {}", "Tasks:".cyan());
            println!("    Dev artifacts:    {}", status.tasks.clean_dev);
            println!("    Package caches:   {}", status.tasks.clean_packages);
            println!("    Docker:           {}", status.tasks.clean_docker);
            println!("    System caches:    {}", status.tasks.clean_system);
            println!("    Browser caches:   {}", status.tasks.clean_browsers);
            println!("    Empty trash:      {}", status.tasks.empty_trash);

            if let Some(last_run) = &status.last_run {
                println!("\n  Last run: {}", last_run);
            }
            println!();
        }

        ScheduleCommand::RunNow => {
            info("Running scheduled tasks...");
            let result = run_scheduled_tasks()?;
            println!(
                "\n{} Scheduled cleanup completed at {}",
                "✓".green(),
                result.timestamp
            );
            println!(
                "  Freed: {}",
                crate::utils::format_size(result.result.bytes_freed).green().bold()
            );
            println!("  Tasks: {}", result.tasks_run.join(", "));
            println!();
        }
    }

    Ok(())
}

fn cmd_config(
    add_dir: Option<PathBuf>,
    remove_dir: Option<PathBuf>,
    show: bool,
    init: bool,
) -> Result<()> {
    if init {
        let config_path = macpurge::get_config_path()?;
        if config_path.exists() {
            warn("Configuration file already exists.");
            if !confirm("Overwrite with defaults?") {
                return Ok(());
            }
        }
        macpurge::init_config()?;
        success(&format!("Configuration initialized at {}", config_path.display()));
        return Ok(());
    }

    let mut config = macpurge::load_config()?;
    let mut modified = false;

    if let Some(ref dir) = add_dir {
        let expanded = macpurge::expand_path(dir);
        if !config.project_dirs.contains(dir) && !config.project_dirs.contains(&expanded) {
            config.project_dirs.push(dir.clone());
            modified = true;
            success(&format!("Added project directory: {}", dir.display()));
        } else {
            info("Directory already in config.");
        }
    }

    if let Some(ref dir) = remove_dir {
        let expanded = macpurge::expand_path(dir);
        let before_len = config.project_dirs.len();
        config.project_dirs.retain(|p| p != dir && p != &expanded);
        if config.project_dirs.len() < before_len {
            modified = true;
            success(&format!("Removed project directory: {}", dir.display()));
        } else {
            warn("Directory not found in config.");
        }
    }

    if modified {
        macpurge::save_config(&config)?;
    }

    if show || (!modified && add_dir.is_none() && remove_dir.is_none()) {
        let config_path = macpurge::get_config_path()?;
        println!("\n{} {}\n", "Config file:".cyan().bold(), config_path.display());

        println!("{}", "Project directories:".cyan());
        if config.project_dirs.is_empty() {
            println!("  (none configured)");
        } else {
            for dir in &config.project_dirs {
                let expanded = macpurge::expand_path(dir);
                let exists = expanded.exists();
                let status = if exists {
                    "✓".green()
                } else {
                    "✗".red()
                };
                println!("  {} {}", status, dir.display());
            }
        }

        println!("\n{}", "Scan settings:".cyan());
        println!("  Max depth: {}", config.scan.max_depth);
        println!("  Include hidden: {}", config.scan.include_hidden);

        println!("\n{}", "Cleanup settings:".cyan());
        println!("  Confirm before delete: {}", config.cleanup.confirm_before_delete);
        println!("  Use trash: {}", config.cleanup.use_trash);

        if let Some(schedule) = &config.schedule {
            println!("\n{}", "Schedule:".cyan());
            println!("  Enabled: {}", schedule.enabled);
            println!("  Cron: {}", schedule.cron);
        }

        println!();
    }

    Ok(())
}
