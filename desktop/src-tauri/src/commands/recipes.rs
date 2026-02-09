//! Recipe commands

use macpurge::{
    CleanOptions, Recipe, RecipeRunResult,
    get_builtin_recipes, run_recipe,
};

#[tauri::command]
pub fn list_recipes() -> Vec<Recipe> {
    get_builtin_recipes()
}

#[tauri::command]
pub fn run_recipe_cmd(id: String, dry_run: bool) -> Result<RecipeRunResult, String> {
    let options = CleanOptions {
        dry_run,
        force: true,
        verbose: false,
        ..Default::default()
    };

    run_recipe(&id, &options).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn estimate_recipe(id: String) -> Result<RecipeRunResult, String> {
    let options = CleanOptions {
        dry_run: true,
        force: true,
        verbose: false,
        ..Default::default()
    };

    run_recipe(&id, &options).map_err(|e| e.to_string())
}
