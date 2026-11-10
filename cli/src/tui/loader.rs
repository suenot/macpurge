//! Blocking data loaders and operations. Run on worker threads, never on the UI thread.

use super::model::{Item, Section};
use anyhow::Result;
use macpurge::ops;
use macpurge::utils::format_size;
use macpurge::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

pub fn tilde(p: &Path) -> String {
    match dirs::home_dir() {
        Some(h) => match p.strip_prefix(&h) {
            Ok(rest) if rest.as_os_str().is_empty() => "~".to_string(),
            Ok(rest) => format!("~/{}", rest.display()),
            Err(_) => p.display().to_string(),
        },
        None => p.display().to_string(),
    }
}

fn sorted(mut v: Vec<Item>) -> Vec<Item> {
    v.sort_by_key(|i| std::cmp::Reverse(i.size));
    v
}

pub fn usage_items(node: &UsageNode) -> Vec<Item> {
    node.children
        .iter()
        .map(|c| Item {
            label: c.name.clone(),
            detail: if c.is_dir { format!("{} files", c.file_count) } else { String::new() },
            path: if c.aggregated_count.is_some() { None } else { Some(c.path.clone()) },
            size: c.size_bytes,
            is_dir: c.is_dir && c.aggregated_count.is_none(),
            ..Default::default()
        })
        .collect()
}

pub fn load_usage() -> Result<UsageNode> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no home directory"))?;
    ops::scan_usage(&home, true, &AtomicBool::new(false))
}

pub fn load(section: Section) -> Result<Vec<Item>> {
    Ok(match section {
        Section::DiskUsage => unreachable!("handled via load_usage"),
        Section::Projects => load_projects()?,
        Section::DevArtifacts => load_dev_artifacts()?,
        Section::Packages => {
            let r = PackageScanner::new().scan_all()?;
            sorted(
                r.managers
                    .iter()
                    .flat_map(|m| {
                        m.cache_paths.iter().filter(|c| c.size_bytes > 0).map(move |c| Item {
                            label: tilde(&c.path),
                            detail: c.description.clone(),
                            tag: m.manager.name().to_string(),
                            path: c.safe_to_clean.then(|| c.path.clone()),
                            size: c.size_bytes,
                            ..Default::default()
                        })
                    })
                    .collect(),
            )
        }
        Section::System => {
            let r = SystemScanner::new().scan_all()?;
            sorted(
                r.categories
                    .iter()
                    .flat_map(|c| {
                        c.paths.iter().filter(|p| p.size_bytes > 0).map(move |p| Item {
                            label: tilde(&p.path),
                            detail: p.description.clone(),
                            tag: c.name.clone(),
                            path: Some(p.path.clone()),
                            size: p.size_bytes,
                            ..Default::default()
                        })
                    })
                    .collect(),
            )
        }
        Section::Applications => {
            let apps = ApplicationScanner::list_apps()?;
            sorted(
                apps.into_iter()
                    .map(|a| Item {
                        label: a.name.clone(),
                        detail: format!(
                            "{} · data {} · cache {}",
                            a.version.clone().unwrap_or_default(),
                            format_size(a.data_size_bytes),
                            format_size(a.cache_size_bytes)
                        ),
                        tag: "app".into(),
                        path: Some(a.path.clone()),
                        size: a.size_bytes,
                        ..Default::default()
                    })
                    .collect(),
            )
        }
        Section::DevTools => {
            let r = DevToolScanner::new().scan_all()?;
            sorted(
                r.tools
                    .iter()
                    .flat_map(|t| {
                        t.cache_paths.iter().filter(|c| c.size_bytes > 0).map(move |c| Item {
                            label: tilde(&c.path),
                            detail: c.description.clone(),
                            tag: c.category.clone(),
                            path: c.safe_to_clean.then(|| c.path.clone()),
                            size: c.size_bytes,
                            ..Default::default()
                        })
                    })
                    .collect(),
            )
        }
        Section::AiModels => {
            let r = AiModelScanner::new().scan_all()?;
            sorted(
                r.tools
                    .iter()
                    .flat_map(|t| {
                        t.models.iter().map(move |m| Item {
                            label: tilde(&m.path),
                            detail: m.description.clone(),
                            tag: m.category.clone(),
                            path: m.safe_to_clean.then(|| m.path.clone()),
                            size: m.size_bytes,
                            ..Default::default()
                        })
                    })
                    .collect(),
            )
        }
        Section::Recipes => get_builtin_recipes()
            .into_iter()
            .map(|r| Item {
                label: r.name,
                detail: r.description,
                tag: r.estimated_savings,
                id: Some(r.id),
                ..Default::default()
            })
            .collect(),
        Section::Schedule => {
            let s = get_schedule_status()?;
            let row = |k: &str, v: String| Item { label: k.into(), detail: v, ..Default::default() };
            vec![
                row("Status", if s.enabled { "enabled".into() } else { "disabled (press Enter to enable)".into() }),
                row("Cron", s.cron),
                row("Last run", s.last_run.unwrap_or_else(|| "never".into())),
                row("Next run", s.next_run.unwrap_or_else(|| "-".into())),
                row("LaunchAgent", s.plist_path),
            ]
        }
        Section::Settings => {
            let c = load_config()?;
            let row = |k: &str, v: String| Item { label: k.into(), detail: v, ..Default::default() };
            let mut v = vec![
                row("Config file", tilde(&get_config_path()?)),
                row("Move to Trash", c.cleanup.use_trash.to_string()),
                row("Archive dir", c.archive_dir.as_ref().map(|p| tilde(p)).unwrap_or_else(|| "not set".into())),
            ];
            for d in &c.project_dirs {
                v.push(row("Project dir", tilde(&expand_path(d))));
            }
            for d in &c.excluded_dirs {
                v.push(row("Excluded", tilde(&expand_path(d))));
            }
            v
        }
    })
}

fn load_dev_artifacts() -> Result<Vec<Item>> {
    let cfg = load_config()?;
    let dirs = get_project_dirs(&cfg);
    let scanner = DirectoryScanner::new(ScanOptions {
        dev_mode: true,
        include_hidden: cfg.scan.include_hidden,
        max_depth: Some(cfg.scan.max_depth),
        min_size: None,
        show_progress: false,
        excluded: get_excluded_dirs(&cfg),
    });
    let mut items = Vec::new();
    for d in dirs.iter().filter(|d| d.exists()) {
        let r = scanner.scan(d)?;
        items.extend(r.entries.into_iter().map(|e| Item {
            label: tilde(&e.path),
            detail: format!("{} files", e.file_count),
            tag: e.file_type.display_name().to_string(),
            path: Some(e.path),
            size: e.size_bytes,
            ..Default::default()
        }));
    }
    Ok(sorted(items))
}

fn load_projects() -> Result<Vec<Item>> {
    let cfg = load_config()?;
    let dirs = get_project_dirs(&cfg);
    let excl = get_excluded_dirs(&cfg);
    let mut items = Vec::new();
    for p in ProjectScanner::scan_projects(&dirs, &excl) {
        let (size, artifacts) = ProjectScanner::calculate_project_size(&p.path);
        let git = if p.has_git {
            match ProjectScanner::get_git_status(&p.path) {
                Some(s) => {
                    let ahead: u32 = s.remotes.iter().map(|r| r.ahead).sum();
                    let mut parts = vec![s.current_branch.unwrap_or_else(|| "detached".into())];
                    if s.has_uncommitted {
                        parts.push(format!("● {} uncommitted", s.uncommitted_count));
                    }
                    if ahead > 0 {
                        parts.push(format!("↑{ahead}"));
                    }
                    parts.join(" ")
                }
                None => "git".into(),
            }
        } else {
            "no git".into()
        };
        items.push(Item {
            label: p.name.clone(),
            detail: format!("{git} · cleanable {}", format_size(artifacts)),
            tag: p.project_type.display_name().to_string(),
            path: Some(p.path.clone()),
            size,
            ..Default::default()
        });
    }
    Ok(sorted(items))
}

pub fn clean(paths: &[PathBuf], dry_run: bool) -> String {
    let r = ops::clean_paths(paths, dry_run);
    let verb = if dry_run { "Would free" } else { "Freed" };
    let mut s = format!("{verb} {} across {} item(s)", format_size(r.bytes_freed), r.items_removed);
    if let Some(e) = r.errors.first() {
        s.push_str(&format!(" · {} error(s): {}", r.errors.len(), e));
    }
    s
}

pub fn run_recipe_msg(id: &str, dry_run: bool) -> String {
    let opts = CleanOptions { dry_run, force: true, verbose: false, use_trash: ops::use_trash(), ..Default::default() };
    match run_recipe(id, &opts) {
        Ok(r) => format!(
            "{} {} of {} steps · {} {}",
            if dry_run { "Dry run:" } else { "Ran" },
            r.steps_completed,
            r.total_steps,
            if dry_run { "would free" } else { "freed" },
            format_size(r.total_result.bytes_freed)
        ),
        Err(e) => format!("Recipe failed: {e}"),
    }
}

pub fn toggle_schedule() -> String {
    match get_schedule_status() {
        Ok(s) if s.enabled => match disable_schedule() {
            Ok(()) => "Schedule disabled".into(),
            Err(e) => format!("Failed: {e}"),
        },
        Ok(_) => {
            let tasks = ScheduleTasks {
                clean_dev: true,
                clean_packages: true,
                clean_docker: true,
                clean_system: true,
                clean_browsers: true,
                empty_trash: true,
            };
            match enable_schedule("0 3 * * 0", &tasks) {
                Ok(()) => "Schedule enabled: Sundays 03:00".into(),
                Err(e) => format!("Failed: {e}"),
            }
        }
        Err(e) => format!("Failed: {e}"),
    }
}

pub fn commit(path: &Path) -> String {
    match ProjectScanner::auto_commit(path) {
        Ok(m) => format!("Committed: {m}"),
        Err(e) => format!("Commit failed: {e}"),
    }
}

pub fn push(path: &Path) -> String {
    match ProjectScanner::push_to_all_remotes(path) {
        Ok(rs) => {
            let ok = rs.iter().filter(|(_, r)| r.is_ok()).count();
            format!("Pushed {ok}/{} remote(s)", rs.len())
        }
        Err(e) => format!("Push failed: {e}"),
    }
}

pub fn archive(path: &Path) -> String {
    match ops::archive_project(path) {
        Ok(e) => format!("Archived {} ({})", e.name, format_size(e.size_bytes)),
        Err(e) => format!("Archive failed: {e}"),
    }
}

pub fn reveal(path: &Path) {
    let _ = std::process::Command::new("open").arg("-R").arg(path).status();
}
