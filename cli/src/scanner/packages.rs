//! Package manager cache scanner
//!
//! Scans caches and global packages for npm, yarn, pnpm, pip, cargo, rustup, brew, etc.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

/// Package manager type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PkgManagerType {
    Npm,
    Yarn,
    Pnpm,
    Pip,
    Cargo,
    Rustup,
    Go,
    Gem,
    Brew,
    Composer,
}

impl PkgManagerType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Npm => "npm",
            Self::Yarn => "yarn",
            Self::Pnpm => "pnpm",
            Self::Pip => "pip",
            Self::Cargo => "cargo",
            Self::Rustup => "rustup",
            Self::Go => "go",
            Self::Gem => "gem",
            Self::Brew => "brew",
            Self::Composer => "composer",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::Npm,
            Self::Yarn,
            Self::Pnpm,
            Self::Pip,
            Self::Cargo,
            Self::Rustup,
            Self::Go,
            Self::Gem,
            Self::Brew,
            Self::Composer,
        ]
    }
}

/// Information about a package manager's cache/data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManagerInfo {
    pub manager: PkgManagerType,
    pub cache_paths: Vec<CachePath>,
    pub total_cache_size: u64,
    pub global_packages: Vec<GlobalPackage>,
    pub is_installed: bool,
}

impl PackageManagerInfo {
    pub fn total_cache_size_human(&self) -> String {
        crate::utils::format_size(self.total_cache_size)
    }
}

/// A cache directory path with size info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePath {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub description: String,
    pub safe_to_clean: bool,
}

impl CachePath {
    pub fn size_human(&self) -> String {
        crate::utils::format_size(self.size_bytes)
    }
}

/// Global package info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalPackage {
    pub name: String,
    pub version: Option<String>,
    pub size_bytes: Option<u64>,
}

/// Result of scanning all package managers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageScanResult {
    pub managers: Vec<PackageManagerInfo>,
    pub total_cache_size: u64,
    pub scan_duration_ms: u64,
}

impl PackageScanResult {
    pub fn total_cache_size_human(&self) -> String {
        crate::utils::format_size(self.total_cache_size)
    }
}

/// Scanner for package manager caches
pub struct PackageScanner {
    home_dir: PathBuf,
}

impl PackageScanner {
    pub fn new() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        Self { home_dir }
    }

    /// Scan all package managers
    pub fn scan_all(&self) -> Result<PackageScanResult> {
        let start = std::time::Instant::now();
        let mut managers = Vec::new();

        for pm in PkgManagerType::all() {
            if let Ok(info) = self.scan_manager(pm) {
                managers.push(info);
            }
        }

        let total_cache_size = managers.iter().map(|m| m.total_cache_size).sum();

        Ok(PackageScanResult {
            managers,
            total_cache_size,
            scan_duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Scan a specific package manager
    pub fn scan_manager(&self, manager: PkgManagerType) -> Result<PackageManagerInfo> {
        match manager {
            PkgManagerType::Npm => self.scan_npm(),
            PkgManagerType::Yarn => self.scan_yarn(),
            PkgManagerType::Pnpm => self.scan_pnpm(),
            PkgManagerType::Pip => self.scan_pip(),
            PkgManagerType::Cargo => self.scan_cargo(),
            PkgManagerType::Rustup => self.scan_rustup(),
            PkgManagerType::Go => self.scan_go(),
            PkgManagerType::Gem => self.scan_gem(),
            PkgManagerType::Brew => self.scan_brew(),
            PkgManagerType::Composer => self.scan_composer(),
        }
    }

    /// Scan npm cache and global packages
    fn scan_npm(&self) -> Result<PackageManagerInfo> {
        let mut cache_paths = Vec::new();
        let mut global_packages = Vec::new();
        let is_installed = which::which("npm").is_ok();

        // npm cache directory
        let npm_cache = self.home_dir.join(".npm");
        if npm_cache.exists() {
            let size = get_dir_size(&npm_cache);
            cache_paths.push(CachePath {
                path: npm_cache,
                size_bytes: size,
                description: "npm cache".to_string(),
                safe_to_clean: true,
            });
        }

        // Global node_modules (if exists in home)
        let global_node_modules = self.home_dir.join("node_modules");
        if global_node_modules.exists() {
            let size = get_dir_size(&global_node_modules);
            cache_paths.push(CachePath {
                path: global_node_modules,
                size_bytes: size,
                description: "Global node_modules (unusual location)".to_string(),
                safe_to_clean: false,
            });
        }

        // Get global packages list
        if is_installed {
            if let Ok(output) = Command::new("npm").args(["ls", "-g", "--depth=0", "--json"]).output() {
                if let Ok(json) = String::from_utf8(output.stdout) {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json) {
                        if let Some(deps) = parsed.get("dependencies").and_then(|d| d.as_object()) {
                            for (name, info) in deps {
                                let version = info.get("version").and_then(|v| v.as_str()).map(String::from);
                                global_packages.push(GlobalPackage {
                                    name: name.clone(),
                                    version,
                                    size_bytes: None,
                                });
                            }
                        }
                    }
                }
            }
        }

        let total_cache_size = cache_paths.iter().map(|c| c.size_bytes).sum();

        Ok(PackageManagerInfo {
            manager: PkgManagerType::Npm,
            cache_paths,
            total_cache_size,
            global_packages,
            is_installed,
        })
    }

    /// Scan yarn cache
    fn scan_yarn(&self) -> Result<PackageManagerInfo> {
        let mut cache_paths = Vec::new();
        let is_installed = which::which("yarn").is_ok();

        // Yarn 1.x cache
        let yarn_cache_v1 = self.home_dir.join(".yarn/cache");
        if yarn_cache_v1.exists() {
            let size = get_dir_size(&yarn_cache_v1);
            cache_paths.push(CachePath {
                path: yarn_cache_v1,
                size_bytes: size,
                description: "Yarn cache".to_string(),
                safe_to_clean: true,
            });
        }

        // Yarn berry cache
        let yarn_cache_berry = self.home_dir.join(".cache/yarn");
        if yarn_cache_berry.exists() {
            let size = get_dir_size(&yarn_cache_berry);
            cache_paths.push(CachePath {
                path: yarn_cache_berry,
                size_bytes: size,
                description: "Yarn berry cache".to_string(),
                safe_to_clean: true,
            });
        }

        // Try to get cache dir from yarn itself
        if is_installed {
            if let Ok(output) = Command::new("yarn").args(["cache", "dir"]).output() {
                if let Ok(dir) = String::from_utf8(output.stdout) {
                    let cache_dir = PathBuf::from(dir.trim());
                    if cache_dir.exists() && !cache_paths.iter().any(|c| c.path == cache_dir) {
                        let size = get_dir_size(&cache_dir);
                        cache_paths.push(CachePath {
                            path: cache_dir,
                            size_bytes: size,
                            description: "Yarn cache (from yarn cache dir)".to_string(),
                            safe_to_clean: true,
                        });
                    }
                }
            }
        }

        let total_cache_size = cache_paths.iter().map(|c| c.size_bytes).sum();

        Ok(PackageManagerInfo {
            manager: PkgManagerType::Yarn,
            cache_paths,
            total_cache_size,
            global_packages: Vec::new(),
            is_installed,
        })
    }

    /// Scan pnpm store
    fn scan_pnpm(&self) -> Result<PackageManagerInfo> {
        let mut cache_paths = Vec::new();
        let is_installed = which::which("pnpm").is_ok();

        // Default pnpm store location
        let pnpm_store = self.home_dir.join(".local/share/pnpm/store");
        if pnpm_store.exists() {
            let size = get_dir_size(&pnpm_store);
            cache_paths.push(CachePath {
                path: pnpm_store,
                size_bytes: size,
                description: "pnpm store".to_string(),
                safe_to_clean: false, // Content-addressable store, be careful
            });
        }

        // Alternative location
        let pnpm_store_alt = self.home_dir.join(".pnpm-store");
        if pnpm_store_alt.exists() {
            let size = get_dir_size(&pnpm_store_alt);
            cache_paths.push(CachePath {
                path: pnpm_store_alt,
                size_bytes: size,
                description: "pnpm store (alt)".to_string(),
                safe_to_clean: false,
            });
        }

        let total_cache_size = cache_paths.iter().map(|c| c.size_bytes).sum();

        Ok(PackageManagerInfo {
            manager: PkgManagerType::Pnpm,
            cache_paths,
            total_cache_size,
            global_packages: Vec::new(),
            is_installed,
        })
    }

    /// Scan pip cache
    fn scan_pip(&self) -> Result<PackageManagerInfo> {
        let mut cache_paths = Vec::new();
        let is_installed = which::which("pip3").is_ok() || which::which("pip").is_ok();

        // pip cache on macOS
        let pip_cache = self.home_dir.join("Library/Caches/pip");
        if pip_cache.exists() {
            let size = get_dir_size(&pip_cache);
            cache_paths.push(CachePath {
                path: pip_cache,
                size_bytes: size,
                description: "pip cache".to_string(),
                safe_to_clean: true,
            });
        }

        // Alternative pip cache location
        let pip_cache_alt = self.home_dir.join(".cache/pip");
        if pip_cache_alt.exists() {
            let size = get_dir_size(&pip_cache_alt);
            cache_paths.push(CachePath {
                path: pip_cache_alt,
                size_bytes: size,
                description: "pip cache (Linux style)".to_string(),
                safe_to_clean: true,
            });
        }

        let total_cache_size = cache_paths.iter().map(|c| c.size_bytes).sum();

        Ok(PackageManagerInfo {
            manager: PkgManagerType::Pip,
            cache_paths,
            total_cache_size,
            global_packages: Vec::new(),
            is_installed,
        })
    }

    /// Scan cargo cache and registry
    fn scan_cargo(&self) -> Result<PackageManagerInfo> {
        let mut cache_paths = Vec::new();
        let is_installed = which::which("cargo").is_ok();

        let cargo_home = self.home_dir.join(".cargo");

        // Registry cache
        let registry = cargo_home.join("registry");
        if registry.exists() {
            let size = get_dir_size(&registry);
            cache_paths.push(CachePath {
                path: registry,
                size_bytes: size,
                description: "Cargo registry (crates index + cache)".to_string(),
                safe_to_clean: true,
            });
        }

        // Git dependencies
        let git = cargo_home.join("git");
        if git.exists() {
            let size = get_dir_size(&git);
            cache_paths.push(CachePath {
                path: git,
                size_bytes: size,
                description: "Cargo git dependencies".to_string(),
                safe_to_clean: true,
            });
        }

        let total_cache_size = cache_paths.iter().map(|c| c.size_bytes).sum();

        Ok(PackageManagerInfo {
            manager: PkgManagerType::Cargo,
            cache_paths,
            total_cache_size,
            global_packages: Vec::new(),
            is_installed,
        })
    }

    /// Scan rustup toolchains
    fn scan_rustup(&self) -> Result<PackageManagerInfo> {
        let mut cache_paths = Vec::new();
        let mut global_packages = Vec::new();
        let is_installed = which::which("rustup").is_ok();

        let rustup_home = self.home_dir.join(".rustup");

        // Toolchains
        let toolchains = rustup_home.join("toolchains");
        if toolchains.exists() {
            // List each toolchain separately
            if let Ok(entries) = std::fs::read_dir(&toolchains) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        let size = get_dir_size(&path);
                        global_packages.push(GlobalPackage {
                            name: name.clone(),
                            version: None,
                            size_bytes: Some(size),
                        });
                    }
                }
            }

            let size = get_dir_size(&toolchains);
            cache_paths.push(CachePath {
                path: toolchains,
                size_bytes: size,
                description: "Rust toolchains".to_string(),
                safe_to_clean: false, // Don't auto-clean toolchains
            });
        }

        // Downloads cache
        let downloads = rustup_home.join("downloads");
        if downloads.exists() {
            let size = get_dir_size(&downloads);
            cache_paths.push(CachePath {
                path: downloads,
                size_bytes: size,
                description: "Rustup downloads cache".to_string(),
                safe_to_clean: true,
            });
        }

        let total_cache_size = cache_paths.iter().map(|c| c.size_bytes).sum();

        Ok(PackageManagerInfo {
            manager: PkgManagerType::Rustup,
            cache_paths,
            total_cache_size,
            global_packages,
            is_installed,
        })
    }

    /// Scan go modules cache
    fn scan_go(&self) -> Result<PackageManagerInfo> {
        let mut cache_paths = Vec::new();
        let is_installed = which::which("go").is_ok();

        // Go modules cache
        let go_mod_cache = self.home_dir.join("go/pkg/mod/cache");
        if go_mod_cache.exists() {
            let size = get_dir_size(&go_mod_cache);
            cache_paths.push(CachePath {
                path: go_mod_cache,
                size_bytes: size,
                description: "Go modules cache".to_string(),
                safe_to_clean: true,
            });
        }

        // Full pkg directory
        let go_pkg = self.home_dir.join("go/pkg");
        if go_pkg.exists() {
            let size = get_dir_size(&go_pkg);
            cache_paths.push(CachePath {
                path: go_pkg,
                size_bytes: size,
                description: "Go packages".to_string(),
                safe_to_clean: false,
            });
        }

        let total_cache_size = cache_paths.iter().map(|c| c.size_bytes).sum();

        Ok(PackageManagerInfo {
            manager: PkgManagerType::Go,
            cache_paths,
            total_cache_size,
            global_packages: Vec::new(),
            is_installed,
        })
    }

    /// Scan gem cache
    fn scan_gem(&self) -> Result<PackageManagerInfo> {
        let mut cache_paths = Vec::new();
        let is_installed = which::which("gem").is_ok();

        // Gem cache
        let gem_cache = self.home_dir.join(".gem");
        if gem_cache.exists() {
            let size = get_dir_size(&gem_cache);
            cache_paths.push(CachePath {
                path: gem_cache,
                size_bytes: size,
                description: "Ruby gems".to_string(),
                safe_to_clean: false,
            });
        }

        let total_cache_size = cache_paths.iter().map(|c| c.size_bytes).sum();

        Ok(PackageManagerInfo {
            manager: PkgManagerType::Gem,
            cache_paths,
            total_cache_size,
            global_packages: Vec::new(),
            is_installed,
        })
    }

    /// Scan homebrew cache
    fn scan_brew(&self) -> Result<PackageManagerInfo> {
        let mut cache_paths = Vec::new();
        let is_installed = which::which("brew").is_ok();

        // Homebrew cache
        let brew_cache = self.home_dir.join("Library/Caches/Homebrew");
        if brew_cache.exists() {
            let size = get_dir_size(&brew_cache);
            cache_paths.push(CachePath {
                path: brew_cache,
                size_bytes: size,
                description: "Homebrew cache".to_string(),
                safe_to_clean: true,
            });
        }

        // Homebrew downloads
        let brew_downloads = self.home_dir.join("Library/Caches/Homebrew/downloads");
        if brew_downloads.exists() && !cache_paths.iter().any(|c| c.path == brew_downloads) {
            let size = get_dir_size(&brew_downloads);
            cache_paths.push(CachePath {
                path: brew_downloads,
                size_bytes: size,
                description: "Homebrew downloads".to_string(),
                safe_to_clean: true,
            });
        }

        let total_cache_size = cache_paths.iter().map(|c| c.size_bytes).sum();

        Ok(PackageManagerInfo {
            manager: PkgManagerType::Brew,
            cache_paths,
            total_cache_size,
            global_packages: Vec::new(),
            is_installed,
        })
    }

    /// Scan composer cache
    fn scan_composer(&self) -> Result<PackageManagerInfo> {
        let mut cache_paths = Vec::new();
        let is_installed = which::which("composer").is_ok();

        // Composer cache
        let composer_cache = self.home_dir.join(".composer/cache");
        if composer_cache.exists() {
            let size = get_dir_size(&composer_cache);
            cache_paths.push(CachePath {
                path: composer_cache,
                size_bytes: size,
                description: "Composer cache".to_string(),
                safe_to_clean: true,
            });
        }

        // Alternative location
        let composer_cache_alt = self.home_dir.join("Library/Caches/composer");
        if composer_cache_alt.exists() {
            let size = get_dir_size(&composer_cache_alt);
            cache_paths.push(CachePath {
                path: composer_cache_alt,
                size_bytes: size,
                description: "Composer cache (macOS)".to_string(),
                safe_to_clean: true,
            });
        }

        let total_cache_size = cache_paths.iter().map(|c| c.size_bytes).sum();

        Ok(PackageManagerInfo {
            manager: PkgManagerType::Composer,
            cache_paths,
            total_cache_size,
            global_packages: Vec::new(),
            is_installed,
        })
    }
}

/// Calculate total size of a directory
fn get_dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}
