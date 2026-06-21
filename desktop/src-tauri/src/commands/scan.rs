//! Scanning commands

use macpurge::{DirectoryScanner, ScanOptions, ScanResult};
use std::path::PathBuf;

/// Scan project directories from config for dev artifacts
#[tauri::command]
pub async fn scan_dev() -> Result<Vec<ScanResult>, String> {
    tokio::task::spawn_blocking(|| {
        let config = macpurge::load_config().map_err(|e| e.to_string())?;
        let project_dirs = macpurge::get_project_dirs(&config);

        if project_dirs.is_empty() {
            return Err("No project directories configured. Add directories in Settings.".to_string());
        }

        let scanner = DirectoryScanner::new(ScanOptions {
            dev_mode: true,
            include_hidden: config.scan.include_hidden,
            max_depth: Some(config.scan.max_depth),
            min_size: config.scan.min_size_bytes,
            show_progress: false,
        });

        let mut results = Vec::new();
        for path in project_dirs {
            if path.exists() {
                match scanner.scan(&path) {
                    Ok(result) => results.push(result),
                    Err(e) => {
                        eprintln!("Failed to scan {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok::<_, String>(results)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Scan a specific path for dev artifacts
#[tauri::command]
pub async fn scan_path(path: String) -> Result<ScanResult, String> {
    tokio::task::spawn_blocking(move || {
        let path = PathBuf::from(&path);

        if !path.exists() {
            return Err(format!("Path does not exist: {}", path.display()));
        }

        let config = macpurge::load_config().unwrap_or_default();

        let scanner = DirectoryScanner::new(ScanOptions {
            dev_mode: true,
            include_hidden: config.scan.include_hidden,
            max_depth: Some(config.scan.max_depth),
            min_size: config.scan.min_size_bytes,
            show_progress: false,
        });

        scanner.scan(&path).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get list of supported dev artifact types
#[tauri::command]
pub fn get_dev_artifact_types() -> Vec<ArtifactType> {
    vec![
        ArtifactType {
            id: "node_modules".to_string(),
            name: "Node.js modules".to_string(),
            description: "node_modules directories".to_string(),
            icon: "📦".to_string(),
        },
        ArtifactType {
            id: "target".to_string(),
            name: "Rust target".to_string(),
            description: "Cargo build output".to_string(),
            icon: "🦀".to_string(),
        },
        ArtifactType {
            id: "venv".to_string(),
            name: "Python venv".to_string(),
            description: "Virtual environments".to_string(),
            icon: "🐍".to_string(),
        },
        ArtifactType {
            id: "__pycache__".to_string(),
            name: "Python cache".to_string(),
            description: "Bytecode cache".to_string(),
            icon: "🐍".to_string(),
        },
        ArtifactType {
            id: ".next".to_string(),
            name: "Next.js cache".to_string(),
            description: "Build cache".to_string(),
            icon: "▲".to_string(),
        },
        ArtifactType {
            id: ".gradle".to_string(),
            name: "Gradle cache".to_string(),
            description: "Build cache".to_string(),
            icon: "🐘".to_string(),
        },
        ArtifactType {
            id: "build".to_string(),
            name: "Build directories".to_string(),
            description: "Generic build output".to_string(),
            icon: "🔨".to_string(),
        },
        ArtifactType {
            id: "dist".to_string(),
            name: "Dist directories".to_string(),
            description: "Distribution output".to_string(),
            icon: "📁".to_string(),
        },
    ]
}

#[derive(serde::Serialize)]
pub struct ArtifactType {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
}
