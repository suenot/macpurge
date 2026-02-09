//! Cleanup recipes — predefined multi-step cleanup workflows

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::cleaners::{
    CleanOptions, CleanResult, Cleaner,
    SystemCacheCleaner, DevToolsCleaner,
    PackageManagerCleaner, DevArtifactsCleaner,
};
use crate::config;

/// A cleanup recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub steps: Vec<RecipeStep>,
    pub estimated_savings: String,
}

/// A step in a recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeStep {
    pub action: RecipeAction,
    pub description: String,
}

/// Actions a recipe can perform
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeAction {
    CleanNodeModules { min_age_days: Option<u32> },
    CleanRustTargets,
    CleanPythonVenvs,
    CleanPackageCaches { managers: Vec<String> },
    CleanBrowserCaches,
    CleanDockerDangling,
    CleanXcodeDerivedData,
    CleanSystemLogs,
    CleanTrash,
    CleanOldDownloads { min_age_days: Option<u32> },
}

/// Result of running a recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeRunResult {
    pub recipe_id: String,
    pub steps_completed: usize,
    pub total_steps: usize,
    pub total_result: CleanResult,
    pub step_results: Vec<StepResult>,
}

/// Result of a single recipe step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub description: String,
    pub result: CleanResult,
    pub success: bool,
}

/// Get all built-in recipes
pub fn get_builtin_recipes() -> Vec<Recipe> {
    vec![
        Recipe {
            id: "quick-dev".to_string(),
            name: "Quick Dev Cleanup".to_string(),
            description: "Clean stale node_modules, Rust targets, and Python venvs from project directories".to_string(),
            category: "dev".to_string(),
            steps: vec![
                RecipeStep {
                    action: RecipeAction::CleanNodeModules { min_age_days: Some(30) },
                    description: "Remove node_modules older than 30 days".to_string(),
                },
                RecipeStep {
                    action: RecipeAction::CleanRustTargets,
                    description: "Remove Rust target/ directories".to_string(),
                },
                RecipeStep {
                    action: RecipeAction::CleanPythonVenvs,
                    description: "Remove Python virtual environments".to_string(),
                },
            ],
            estimated_savings: "5-30 GB".to_string(),
        },
        Recipe {
            id: "package-caches".to_string(),
            name: "Clean Package Caches".to_string(),
            description: "Clear all package manager caches (npm, cargo, pip, brew)".to_string(),
            category: "system".to_string(),
            steps: vec![
                RecipeStep {
                    action: RecipeAction::CleanPackageCaches {
                        managers: vec!["npm".to_string(), "cargo".to_string(), "pip".to_string(), "brew".to_string()],
                    },
                    description: "Clean npm, cargo, pip, and Homebrew caches".to_string(),
                },
            ],
            estimated_savings: "2-8 GB".to_string(),
        },
        Recipe {
            id: "browser-cleanup".to_string(),
            name: "Browser Cleanup".to_string(),
            description: "Clear browser caches for Chrome, Safari, Firefox, and others".to_string(),
            category: "system".to_string(),
            steps: vec![
                RecipeStep {
                    action: RecipeAction::CleanBrowserCaches,
                    description: "Clean all browser caches".to_string(),
                },
            ],
            estimated_savings: "1-5 GB".to_string(),
        },
        Recipe {
            id: "docker-cleanup".to_string(),
            name: "Docker Cleanup".to_string(),
            description: "Remove dangling Docker images, stopped containers, and unused volumes".to_string(),
            category: "devtools".to_string(),
            steps: vec![
                RecipeStep {
                    action: RecipeAction::CleanDockerDangling,
                    description: "Prune dangling Docker resources".to_string(),
                },
            ],
            estimated_savings: "5-30 GB".to_string(),
        },
        Recipe {
            id: "xcode-cleanup".to_string(),
            name: "Xcode Cleanup".to_string(),
            description: "Clean Xcode DerivedData and build caches".to_string(),
            category: "devtools".to_string(),
            steps: vec![
                RecipeStep {
                    action: RecipeAction::CleanXcodeDerivedData,
                    description: "Remove Xcode DerivedData".to_string(),
                },
            ],
            estimated_savings: "10-50 GB".to_string(),
        },
        Recipe {
            id: "system-cleanup".to_string(),
            name: "System Cleanup".to_string(),
            description: "Clean system logs, trash, and old downloads".to_string(),
            category: "system".to_string(),
            steps: vec![
                RecipeStep {
                    action: RecipeAction::CleanSystemLogs,
                    description: "Clean system and application logs".to_string(),
                },
                RecipeStep {
                    action: RecipeAction::CleanTrash,
                    description: "Empty the Trash".to_string(),
                },
                RecipeStep {
                    action: RecipeAction::CleanOldDownloads { min_age_days: Some(30) },
                    description: "Remove downloads older than 30 days".to_string(),
                },
            ],
            estimated_savings: "1-10 GB".to_string(),
        },
    ]
}

/// Run a recipe by ID
pub fn run_recipe(id: &str, options: &CleanOptions) -> Result<RecipeRunResult> {
    let recipes = get_builtin_recipes();
    let recipe = recipes.iter().find(|r| r.id == id)
        .ok_or_else(|| anyhow::anyhow!("Recipe not found: {}", id))?;

    let mut total_result = CleanResult::new();
    let mut step_results = Vec::new();
    let mut steps_completed = 0;

    for step in &recipe.steps {
        match execute_recipe_action(&step.action, options) {
            Ok(result) => {
                let success = result.errors.is_empty();
                total_result.merge(result.clone());
                step_results.push(StepResult {
                    description: step.description.clone(),
                    result,
                    success,
                });
                steps_completed += 1;
            }
            Err(e) => {
                let mut err_result = CleanResult::new();
                err_result.errors.push(e.to_string());
                step_results.push(StepResult {
                    description: step.description.clone(),
                    result: err_result,
                    success: false,
                });
            }
        }
    }

    Ok(RecipeRunResult {
        recipe_id: recipe.id.clone(),
        steps_completed,
        total_steps: recipe.steps.len(),
        total_result,
        step_results,
    })
}

/// Execute a single recipe action
fn execute_recipe_action(action: &RecipeAction, options: &CleanOptions) -> Result<CleanResult> {
    match action {
        RecipeAction::CleanNodeModules { min_age_days } => {
            let cfg = config::load_config()?;
            let dirs = config::get_project_dirs(&cfg);
            let mut opts = options.clone();
            opts.min_age_days = min_age_days.map(|d| d as u64);

            let mut total = CleanResult::new();
            for dir in dirs {
                if dir.exists() {
                    let cleaner = DevArtifactsCleaner::node_modules(dir);
                    if let Ok(r) = cleaner.clean(&opts) {
                        total.merge(r);
                    }
                }
            }
            Ok(total)
        }
        RecipeAction::CleanRustTargets => {
            let cfg = config::load_config()?;
            let dirs = config::get_project_dirs(&cfg);

            let mut total = CleanResult::new();
            for dir in dirs {
                if dir.exists() {
                    let cleaner = DevArtifactsCleaner::rust_target(dir);
                    if let Ok(r) = cleaner.clean(options) {
                        total.merge(r);
                    }
                }
            }
            Ok(total)
        }
        RecipeAction::CleanPythonVenvs => {
            let cfg = config::load_config()?;
            let dirs = config::get_project_dirs(&cfg);

            let mut total = CleanResult::new();
            for dir in dirs {
                if dir.exists() {
                    let cleaner = DevArtifactsCleaner::python_venv(dir);
                    if let Ok(r) = cleaner.clean(options) {
                        total.merge(r);
                    }
                }
            }
            Ok(total)
        }
        RecipeAction::CleanPackageCaches { managers } => {
            let mut total = CleanResult::new();
            for mgr in managers {
                let cleaner = match mgr.as_str() {
                    "npm" => PackageManagerCleaner::npm(),
                    "cargo" => PackageManagerCleaner::cargo(),
                    "pip" => PackageManagerCleaner::pip(),
                    "brew" => PackageManagerCleaner::homebrew(),
                    _ => continue,
                };
                if let Ok(r) = cleaner.clean(options) {
                    total.merge(r);
                }
            }
            Ok(total)
        }
        RecipeAction::CleanBrowserCaches => {
            SystemCacheCleaner::browser_cache().clean(options)
        }
        RecipeAction::CleanDockerDangling => {
            DevToolsCleaner::docker_dangling().clean(options)
        }
        RecipeAction::CleanXcodeDerivedData => {
            DevToolsCleaner::xcode_derived_data().clean(options)
        }
        RecipeAction::CleanSystemLogs => {
            SystemCacheCleaner::system_logs().clean(options)
        }
        RecipeAction::CleanTrash => {
            SystemCacheCleaner::trash().clean(options)
        }
        RecipeAction::CleanOldDownloads { min_age_days } => {
            let mut opts = options.clone();
            opts.min_age_days = min_age_days.map(|d| d as u64);
            SystemCacheCleaner::downloads_old().clean(&opts)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_recipes() {
        let recipes = get_builtin_recipes();
        assert_eq!(recipes.len(), 6);
        assert_eq!(recipes[0].id, "quick-dev");
    }

    #[test]
    fn test_recipe_ids_unique() {
        let recipes = get_builtin_recipes();
        let mut ids: Vec<&str> = recipes.iter().map(|r| r.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), recipes.len());
    }
}
