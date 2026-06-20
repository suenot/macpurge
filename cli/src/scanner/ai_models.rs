//! AI / neural-network model scanner.
//!
//! Finds large model artifacts left behind by local LLM / diffusion tooling:
//! Ollama, LM Studio, the Hugging Face cache (where MLX, paroquant and
//! diffusers models are downloaded), and ComfyUI.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::calculate_dir_size;

/// Types of AI model tooling we know how to scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiToolType {
    Ollama,
    LmStudio,
    HuggingFace,
    ComfyUi,
}

impl AiToolType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Ollama => "Ollama",
            Self::LmStudio => "LM Studio",
            Self::HuggingFace => "Hugging Face Cache",
            Self::ComfyUi => "ComfyUI",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![Self::Ollama, Self::LmStudio, Self::HuggingFace, Self::ComfyUi]
    }
}

/// A single model artifact path that can be removed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiModelPath {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub description: String,
    /// Sub-category used for UI badges: ollama, lmstudio, mlx, paroquant,
    /// diffusers, huggingface, comfyui, xet.
    pub category: String,
    pub safe_to_clean: bool,
}

impl AiModelPath {
    pub fn size_human(&self) -> String {
        crate::utils::format_size(self.size_bytes)
    }
}

/// Information about a specific AI tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiToolInfo {
    pub tool: AiToolType,
    pub is_installed: bool,
    pub models: Vec<AiModelPath>,
    pub total_size: u64,
}

impl AiToolInfo {
    pub fn total_size_human(&self) -> String {
        crate::utils::format_size(self.total_size)
    }
}

/// Result of scanning all AI tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiModelScanResult {
    pub tools: Vec<AiToolInfo>,
    pub total_size: u64,
    pub scan_duration_ms: u64,
}

impl AiModelScanResult {
    pub fn total_size_human(&self) -> String {
        crate::utils::format_size(self.total_size)
    }
}

/// Scanner for AI model caches.
pub struct AiModelScanner {
    home_dir: PathBuf,
}

impl AiModelScanner {
    pub fn new() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        Self { home_dir }
    }

    /// Scan all AI tools.
    pub fn scan_all(&self) -> Result<AiModelScanResult> {
        let start = std::time::Instant::now();
        let mut tools = Vec::new();

        for tool_type in AiToolType::all() {
            if let Ok(info) = self.scan_tool(tool_type) {
                tools.push(info);
            }
        }

        let total_size = tools.iter().map(|t| t.total_size).sum();

        Ok(AiModelScanResult {
            tools,
            total_size,
            scan_duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Scan a specific AI tool.
    pub fn scan_tool(&self, tool: AiToolType) -> Result<AiToolInfo> {
        let models = match tool {
            AiToolType::Ollama => self.scan_ollama(),
            AiToolType::LmStudio => self.scan_lmstudio(),
            AiToolType::HuggingFace => self.scan_huggingface(),
            AiToolType::ComfyUi => self.scan_comfyui(),
        };

        let is_installed = self.is_installed(tool);
        let total_size = models.iter().map(|m| m.size_bytes).sum();

        Ok(AiToolInfo {
            tool,
            is_installed,
            models,
            total_size,
        })
    }

    fn is_installed(&self, tool: AiToolType) -> bool {
        match tool {
            AiToolType::Ollama => {
                which::which("ollama").is_ok() || self.home_dir.join(".ollama").exists()
            }
            AiToolType::LmStudio => {
                self.home_dir.join(".lmstudio").exists()
                    || self.home_dir.join("Library/Application Support/LM Studio").exists()
            }
            AiToolType::HuggingFace => self.home_dir.join(".cache/huggingface").exists(),
            AiToolType::ComfyUi => self.comfyui_roots().iter().any(|p| p.exists()),
        }
    }

    // ── Ollama ────────────────────────────────────────────────
    // Blobs are content-addressed and shared between models, so per-model
    // file deletion is unsafe. We expose the whole model store as one unit.
    fn scan_ollama(&self) -> Vec<AiModelPath> {
        let mut out = Vec::new();
        let models_dir = self.home_dir.join(".ollama/models");
        if !models_dir.exists() {
            return out;
        }

        let (size, _) = calculate_dir_size(&models_dir);
        if size == 0 {
            return out;
        }

        let count = self.count_ollama_models(&models_dir.join("manifests"));
        let desc = if count > 0 {
            format!("All Ollama models ({} pulled)", count)
        } else {
            "All Ollama models".to_string()
        };

        out.push(AiModelPath {
            path: models_dir,
            size_bytes: size,
            description: desc,
            category: "ollama".to_string(),
            safe_to_clean: true,
        });
        out
    }

    fn count_ollama_models(&self, manifests: &Path) -> usize {
        // Each tag file under manifests/<registry>/<ns>/<model>/<tag> is a model.
        fn walk(dir: &Path, count: &mut usize) {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for e in entries.flatten() {
                    let p = e.path();
                    if p.is_dir() {
                        walk(&p, count);
                    } else if p.is_file() {
                        *count += 1;
                    }
                }
            }
        }
        let mut count = 0;
        if manifests.exists() {
            walk(manifests, &mut count);
        }
        count
    }

    // ── LM Studio ─────────────────────────────────────────────
    fn scan_lmstudio(&self) -> Vec<AiModelPath> {
        let mut out = Vec::new();
        let base = self.home_dir.join(".lmstudio/models");
        if !base.exists() {
            return out;
        }

        // Layout is models/<publisher>/<model>/...
        if let Ok(publishers) = std::fs::read_dir(&base) {
            for pub_entry in publishers.flatten() {
                let pub_path = pub_entry.path();
                if !pub_path.is_dir() {
                    continue;
                }
                let publisher = pub_entry.file_name().to_string_lossy().to_string();
                if let Ok(models) = std::fs::read_dir(&pub_path) {
                    for model_entry in models.flatten() {
                        let model_path = model_entry.path();
                        if !model_path.is_dir() {
                            continue;
                        }
                        let model = model_entry.file_name().to_string_lossy().to_string();
                        let (size, _) = calculate_dir_size(&model_path);
                        if size == 0 {
                            continue;
                        }
                        let category = if is_mlx(&model) { "mlx" } else { "lmstudio" };
                        out.push(AiModelPath {
                            path: model_path,
                            size_bytes: size,
                            description: format!("{}/{}", publisher, model),
                            category: category.to_string(),
                            safe_to_clean: true,
                        });
                    }
                }
            }
        }
        out
    }

    // ── Hugging Face cache (MLX, paroquant, diffusers, …) ──────
    fn scan_huggingface(&self) -> Vec<AiModelPath> {
        let mut out = Vec::new();
        let hub = self.home_dir.join(".cache/huggingface/hub");
        if hub.exists() {
            if let Ok(entries) = std::fs::read_dir(&hub) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_dir() {
                        continue;
                    }
                    let name = entry.file_name().to_string_lossy().to_string();
                    // Skip control dirs and datasets (we only purge models here).
                    if name.starts_with('.') || name.starts_with("datasets--") {
                        continue;
                    }
                    let (size, _) = calculate_dir_size(&path);
                    if size == 0 {
                        continue;
                    }
                    let repo = pretty_hf_repo(&name);
                    out.push(AiModelPath {
                        path,
                        size_bytes: size,
                        description: repo.clone(),
                        category: hf_category(&repo),
                        safe_to_clean: true,
                    });
                }
            }
        }

        // Xet dedup block store — shared backing storage for hub snapshots.
        let xet = self.home_dir.join(".cache/huggingface/xet");
        if xet.exists() {
            let (size, _) = calculate_dir_size(&xet);
            if size > 0 {
                out.push(AiModelPath {
                    path: xet,
                    size_bytes: size,
                    description: "Hugging Face Xet dedup block store".to_string(),
                    category: "xet".to_string(),
                    safe_to_clean: true,
                });
            }
        }

        out
    }

    // ── ComfyUI ───────────────────────────────────────────────
    fn comfyui_roots(&self) -> Vec<PathBuf> {
        vec![
            self.home_dir.join("Documents/ComfyUI"),
            self.home_dir.join("ComfyUI"),
            self.home_dir.join("comfyui"),
            self.home_dir.join("Desktop/ComfyUI"),
            self.home_dir.join("Downloads/ComfyUI"),
        ]
    }

    fn scan_comfyui(&self) -> Vec<AiModelPath> {
        let mut out = Vec::new();
        for root in self.comfyui_roots() {
            let models = root.join("models");
            if !models.exists() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(&models) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_dir() {
                        continue;
                    }
                    let (size, _) = calculate_dir_size(&path);
                    if size == 0 {
                        continue;
                    }
                    let folder = entry.file_name().to_string_lossy().to_string();
                    out.push(AiModelPath {
                        path,
                        size_bytes: size,
                        description: format!("ComfyUI {} models", folder),
                        category: "comfyui".to_string(),
                        safe_to_clean: true,
                    });
                }
            }
        }
        out
    }
}

impl Default for AiModelScanner {
    fn default() -> Self {
        Self::new()
    }
}

/// `models--Qwen--Qwen3-8B` → `Qwen/Qwen3-8B`
fn pretty_hf_repo(dir_name: &str) -> String {
    let stripped = dir_name.strip_prefix("models--").unwrap_or(dir_name);
    stripped.replace("--", "/")
}

fn is_mlx(name: &str) -> bool {
    let n = name.to_lowercase();
    n.contains("mlx")
}

fn hf_category(repo: &str) -> String {
    let r = repo.to_lowercase();
    if r.contains("paro") || r.starts_with("z-lab/") {
        "paroquant"
    } else if r.contains("vmlx") {
        "vmlx"
    } else if r.contains("mlx") {
        "mlx"
    } else if r.contains("stable-diffusion") || r.contains("flux") || r.contains("diffus") {
        "diffusers"
    } else {
        "huggingface"
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_tool_all() {
        assert_eq!(AiToolType::all().len(), 4);
    }

    #[test]
    fn test_pretty_hf_repo() {
        assert_eq!(pretty_hf_repo("models--Qwen--Qwen3-8B"), "Qwen/Qwen3-8B");
        assert_eq!(pretty_hf_repo("dealignai"), "dealignai");
    }

    #[test]
    fn test_hf_category() {
        assert_eq!(hf_category("z-lab/Qwen3.5-27B-PARO"), "paroquant");
        assert_eq!(hf_category("lmstudio-community/Qwen3-MLX-4bit"), "mlx");
        assert_eq!(hf_category("stable-diffusion-v1-5/stable-diffusion-v1-5"), "diffusers");
        assert_eq!(hf_category("Qwen/Qwen3-8B"), "huggingface");
    }

    #[test]
    fn test_scanner_creation() {
        let scanner = AiModelScanner::new();
        assert!(scanner.scan_all().is_ok());
    }
}
