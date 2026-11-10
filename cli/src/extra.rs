//! Extra subcommands that close the gap with the desktop app:
//! `usage`, `projects`, `clean-*` for packages/devtools/AI models/system.

use anyhow::{bail, Result};
use clap::Subcommand;
use colored::Colorize;
use macpurge::ops;
use macpurge::utils::format_size;
use macpurge::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

#[derive(Subcommand)]
pub enum ProjectsAction {
    /// List projects from configured directories with sizes and git state
    List {
        /// Output format (text, json)
        #[arg(long, short, default_value = "text")]
        output: String,
    },
    /// Git status of one project (or all projects when no path is given)
    Status { path: Option<PathBuf> },
    /// Stage everything and commit with an automatic message
    Commit { path: PathBuf },
    /// Push to a remote (all remotes when --remote is omitted)
    Push {
        path: PathBuf,
        #[arg(long)]
        remote: Option<String>,
    },
    /// Archive projects into the configured archive directory
    Archive {
        paths: Vec<PathBuf>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, short)]
        yes: bool,
    },
    /// List archived projects
    Archives,
    /// Restore an archived project by name
    Restore { name: String },
    /// Move a project to the Trash
    Delete {
        path: PathBuf,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, short)]
        yes: bool,
    },
}

fn confirm_or_yes(yes: bool, msg: &str) -> bool {
    yes || macpurge::confirm(msg)
}

fn bar(frac: f64, width: usize) -> String {
    let n = ((frac.clamp(0.0, 1.0)) * width as f64).round() as usize;
    format!("{}{}", "█".repeat(n), "░".repeat(width - n))
}

fn report(res: &CleanResult, dry_run: bool) {
    let verb = if dry_run { "Would free" } else { "Freed" };
    println!(
        "{} {} across {} item(s)",
        verb.green().bold(),
        format_size(res.bytes_freed).bold(),
        res.items_removed
    );
    for e in &res.errors {
        println!("  {} {}", "!".red(), e);
    }
}

// ---------- usage ----------

pub fn cmd_usage(path: Option<PathBuf>, depth: usize, top: usize, output: String) -> Result<()> {
    let root = match path {
        Some(p) => p,
        None => dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home directory"))?,
    };
    let tree = ops::scan_usage(&root, true, &AtomicBool::new(false))?;
    if output == "json" {
        let payload = tree.to_payload(depth, top, 0);
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }
    println!(
        "{} {}  {} files\n",
        root.display().to_string().bold(),
        format_size(tree.size_bytes).green().bold(),
        tree.file_count
    );
    fn walk(n: &UsageNode, total: u64, level: usize, depth: usize, top: usize) {
        for c in n.children.iter().take(top) {
            let frac = if total == 0 { 0.0 } else { c.size_bytes as f64 / total as f64 };
            println!(
                "{}{} {:>10}  {}{}",
                "  ".repeat(level),
                bar(frac, 12).truecolor(245, 158, 11),
                format_size(c.size_bytes),
                c.name,
                if c.is_dir { "/" } else { "" }
            );
            if c.is_dir && level + 1 < depth {
                walk(c, total, level + 1, depth, top);
            }
        }
    }
    walk(&tree, tree.size_bytes, 0, depth, top);
    Ok(())
}

// ---------- projects ----------

fn project_dirs() -> Result<(Vec<PathBuf>, Vec<PathBuf>)> {
    let cfg = load_config()?;
    let dirs = get_project_dirs(&cfg);
    if dirs.is_empty() {
        bail!("No project directories configured. Run: macpurge config --add-dir ~/projects");
    }
    Ok((dirs, get_excluded_dirs(&cfg)))
}

fn print_git(path: &Path) {
    match ProjectScanner::get_git_status(path) {
        None => println!("  {} not a git repository", "-".dimmed()),
        Some(s) => {
            let branch = s.current_branch.clone().unwrap_or_else(|| "(detached)".into());
            let dirty = if s.has_uncommitted {
                format!("{} uncommitted", s.uncommitted_count).yellow().to_string()
            } else {
                "clean".green().to_string()
            };
            println!("  branch {}  {}", branch.bold(), dirty);
            for r in &s.remotes {
                println!("    {} ahead {} behind {}  {}", r.name, r.ahead, r.behind, r.url.dimmed());
            }
        }
    }
}

pub fn cmd_projects(action: ProjectsAction) -> Result<()> {
    match action {
        ProjectsAction::List { output } => {
            let (dirs, excl) = project_dirs()?;
            let mut projects = ProjectScanner::scan_projects(&dirs, &excl);
            for p in projects.iter_mut() {
                let (size, artifacts) = ProjectScanner::calculate_project_size(&p.path);
                p.size_bytes = Some(size);
                p.artifact_size = Some(artifacts);
            }
            projects.sort_by_key(|p| std::cmp::Reverse(p.size_bytes.unwrap_or(0)));
            if output == "json" {
                println!("{}", serde_json::to_string_pretty(&projects)?);
                return Ok(());
            }
            for p in &projects {
                println!(
                    "{:>10} {:>10}  {:<8} {}  {}",
                    format_size(p.size_bytes.unwrap_or(0)).bold(),
                    format_size(p.artifact_size.unwrap_or(0)).truecolor(245, 158, 11),
                    p.project_type.display_name(),
                    p.name,
                    p.path.display().to_string().dimmed()
                );
            }
            println!("\n{} projects (columns: total, cleanable artifacts)", projects.len());
        }
        ProjectsAction::Status { path } => {
            let paths = match path {
                Some(p) => vec![p],
                None => {
                    let (dirs, excl) = project_dirs()?;
                    ProjectScanner::scan_projects(&dirs, &excl).into_iter().filter(|p| p.has_git).map(|p| p.path).collect()
                }
            };
            for p in paths {
                println!("{}", p.display().to_string().bold());
                print_git(&p);
            }
        }
        ProjectsAction::Commit { path } => {
            let msg = ProjectScanner::auto_commit(&path)?;
            println!("{} {}", "Committed:".green().bold(), msg);
        }
        ProjectsAction::Push { path, remote } => match remote {
            Some(r) => {
                ProjectScanner::push_to_remote(&path, &r)?;
                println!("{} {}", "Pushed to".green().bold(), r);
            }
            None => {
                for (name, res) in ProjectScanner::push_to_all_remotes(&path)? {
                    match res {
                        Ok(()) => println!("{} {}", "pushed".green(), name),
                        Err(e) => println!("{} {}: {}", "failed".red(), name, e),
                    }
                }
            }
        },
        ProjectsAction::Archive { paths, dry_run, yes } => {
            if paths.is_empty() {
                bail!("Give at least one project path");
            }
            for p in &paths {
                if dry_run {
                    println!("Would archive {}", p.display());
                    continue;
                }
                if !confirm_or_yes(yes, &format!("Archive {} (source goes to the Trash)?", p.display())) {
                    continue;
                }
                let e = ops::archive_project(p)?;
                println!("{} {} -> {} ({})", "Archived".green().bold(), e.name, e.archived_path.display(), format_size(e.size_bytes));
            }
        }
        ProjectsAction::Archives => {
            let list = ops::list_archives()?;
            for e in &list {
                println!("{:>10}  {}  {}", format_size(e.size_bytes), e.name.bold(), e.original_path.display().to_string().dimmed());
            }
            println!("\n{} archived project(s)", list.len());
        }
        ProjectsAction::Restore { name } => {
            let dest = ops::restore_archive(&name)?;
            println!("{} {}", "Restored to".green().bold(), dest.display());
        }
        ProjectsAction::Delete { path, dry_run, yes } => {
            if !dry_run && !confirm_or_yes(yes, &format!("Move {} to the Trash?", path.display())) {
                return Ok(());
            }
            report(&ops::clean_paths(&[path], dry_run), dry_run);
        }
    }
    Ok(())
}

// ---------- clean: arbitrary paths, packages, devtools, ai, system ----------

pub fn cmd_clean_path(paths: Vec<PathBuf>, dry_run: bool, yes: bool) -> Result<()> {
    if paths.is_empty() {
        bail!("Give at least one path");
    }
    if !dry_run && !confirm_or_yes(yes, &format!("Move {} path(s) to the Trash?", paths.len())) {
        return Ok(());
    }
    report(&ops::clean_paths(&paths, dry_run), dry_run);
    Ok(())
}

pub fn cmd_clean_packages(manager: Option<String>, dry_run: bool, yes: bool) -> Result<()> {
    let scanner = PackageScanner::new();
    let infos = match manager {
        Some(m) => vec![scanner.scan_manager(ops::parse_pkg_manager(&m)?)?],
        None => scanner.scan_all()?.managers,
    };
    let paths: Vec<PathBuf> = infos
        .iter()
        .flat_map(|i| i.cache_paths.iter().filter(|c| c.safe_to_clean && c.size_bytes > 0).map(|c| c.path.clone()))
        .collect();
    let total: u64 = infos.iter().flat_map(|i| i.cache_paths.iter().filter(|c| c.safe_to_clean)).map(|c| c.size_bytes).sum();
    println!("{} cache path(s), {}", paths.len(), format_size(total));
    if paths.is_empty() {
        return Ok(());
    }
    if !dry_run && !confirm_or_yes(yes, "Move these caches to the Trash?") {
        return Ok(());
    }
    report(&ops::clean_paths(&paths, dry_run), dry_run);
    Ok(())
}

pub fn cmd_clean_devtools(tool: Option<String>, category: Option<String>, dry_run: bool, yes: bool) -> Result<()> {
    let scanner = DevToolScanner::new();
    let infos = match tool {
        Some(t) => vec![scanner.scan_tool(ops::parse_devtool(&t)?)?],
        None => scanner.scan_all()?.tools,
    };
    let paths: Vec<PathBuf> = infos
        .iter()
        .flat_map(|i| i.cache_paths.iter())
        .filter(|c| c.safe_to_clean && c.size_bytes > 0)
        .filter(|c| category.as_ref().map_or(true, |k| c.category.eq_ignore_ascii_case(k)))
        .map(|c| c.path.clone())
        .collect();
    println!("{} dev tool cache path(s)", paths.len());
    if paths.is_empty() {
        return Ok(());
    }
    if !dry_run && !confirm_or_yes(yes, "Move these to the Trash?") {
        return Ok(());
    }
    report(&ops::clean_paths(&paths, dry_run), dry_run);
    Ok(())
}

pub fn cmd_clean_ai(tool: Option<String>, model: Option<String>, dry_run: bool, yes: bool) -> Result<()> {
    let scanner = AiModelScanner::new();
    let infos = match tool {
        Some(t) => vec![scanner.scan_tool(ops::parse_ai_tool(&t)?)?],
        None => scanner.scan_all()?.tools,
    };
    let Some(needle) = model else {
        bail!("Refusing to delete every model. Pick with --model <substring of path or name>; list them with `macpurge scan-ai`");
    };
    let needle = needle.to_lowercase();
    let paths: Vec<PathBuf> = infos
        .iter()
        .flat_map(|i| i.models.iter())
        .filter(|m| m.path.to_string_lossy().to_lowercase().contains(&needle))
        .map(|m| m.path.clone())
        .collect();
    if paths.is_empty() {
        bail!("No model matches '{needle}'");
    }
    for p in &paths {
        println!("  {}", p.display());
    }
    if !dry_run && !confirm_or_yes(yes, &format!("Move {} model path(s) to the Trash?", paths.len())) {
        return Ok(());
    }
    report(&ops::clean_paths(&paths, dry_run), dry_run);
    Ok(())
}

pub fn cmd_clean_system(category: String, min_age_days: Option<u64>, dry_run: bool, yes: bool) -> Result<()> {
    let kind = ops::parse_system_category(&category)?;
    if !dry_run && !confirm_or_yes(yes, &format!("Clean {}?", kind.name())) {
        return Ok(());
    }
    let opts = CleanOptions { dry_run, force: true, verbose: false, min_age_days, use_trash: ops::use_trash() };
    let res = SystemCacheCleaner::new(kind).clean(&opts)?;
    report(&res, dry_run);
    Ok(())
}
