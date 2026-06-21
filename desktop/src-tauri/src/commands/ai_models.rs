//! AI / neural-network model scanning and cleaning commands.

use macpurge::{
    clean_ai_model, AiModelScanResult, AiModelScanner, AiToolInfo, AiToolType, CleanOptions,
    CleanResult,
};
use std::path::PathBuf;

#[tauri::command]
pub fn scan_ai_models() -> Result<AiModelScanResult, String> {
    let scanner = AiModelScanner::new();
    scanner.scan_all().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn scan_ai_tool(tool: String) -> Result<AiToolInfo, String> {
    let scanner = AiModelScanner::new();

    let tool_type = match tool.to_lowercase().as_str() {
        "ollama" => AiToolType::Ollama,
        "lmstudio" | "lm_studio" | "lm-studio" => AiToolType::LmStudio,
        "huggingface" | "hf" => AiToolType::HuggingFace,
        "comfyui" | "comfy" => AiToolType::ComfyUi,
        _ => return Err(format!("Unknown AI tool: {}", tool)),
    };

    scanner.scan_tool(tool_type).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clean_ai_model_path(path: String, dry_run: bool) -> Result<CleanResult, String> {
    // Honor the configured deletion mode (move to Trash vs. permanent delete).
    let use_trash = macpurge::load_config()
        .map(|c| c.cleanup.use_trash)
        .unwrap_or(true);
    let options = CleanOptions {
        dry_run,
        force: true,
        verbose: false,
        use_trash,
        ..Default::default()
    };

    clean_ai_model(PathBuf::from(&path), &options).map_err(|e| e.to_string())
}
