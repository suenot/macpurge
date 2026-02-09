use serde::{Deserialize, Serialize};
use std::path::Path;

/// Types of files/directories that can be cleaned
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileType {
    // Development artifacts
    NodeModules,
    PythonVenv,
    PythonCache,
    RustTarget,
    GoVendor,
    GradleBuild,
    MavenTarget,
    RubyVendor,
    PhpVendor,
    DotNetBin,
    DotNetObj,
    ElixirBuild,
    ElixirDeps,
    HaskellStackWork,

    // Package manager caches
    NpmCache,
    CargoCache,
    HomebrewCache,
    PipCache,
    GradleCache,
    MavenCache,

    // System caches
    SystemCache,
    UserCache,
    BrowserCache,

    // Application remnants
    AppSupport,
    AppPreferences,
    AppCache,
    LaunchAgent,
    LaunchDaemon,

    // Other
    LogFile,
    TempFile,
    DsStore,
    Unknown,
}

impl FileType {
    /// Get display name for the file type
    pub fn display_name(&self) -> &'static str {
        match self {
            FileType::NodeModules => "Node.js modules",
            FileType::PythonVenv => "Python virtual environment",
            FileType::PythonCache => "Python cache",
            FileType::RustTarget => "Rust build artifacts",
            FileType::GoVendor => "Go vendor directory",
            FileType::GradleBuild => "Gradle build",
            FileType::MavenTarget => "Maven target",
            FileType::RubyVendor => "Ruby vendor",
            FileType::PhpVendor => "PHP vendor",
            FileType::DotNetBin => ".NET bin directory",
            FileType::DotNetObj => ".NET obj directory",
            FileType::ElixirBuild => "Elixir build",
            FileType::ElixirDeps => "Elixir dependencies",
            FileType::HaskellStackWork => "Haskell Stack work",
            FileType::NpmCache => "npm cache",
            FileType::CargoCache => "Cargo cache",
            FileType::HomebrewCache => "Homebrew cache",
            FileType::PipCache => "pip cache",
            FileType::GradleCache => "Gradle cache",
            FileType::MavenCache => "Maven cache",
            FileType::SystemCache => "System cache",
            FileType::UserCache => "User cache",
            FileType::BrowserCache => "Browser cache",
            FileType::AppSupport => "Application support",
            FileType::AppPreferences => "Application preferences",
            FileType::AppCache => "Application cache",
            FileType::LaunchAgent => "Launch agent",
            FileType::LaunchDaemon => "Launch daemon",
            FileType::LogFile => "Log file",
            FileType::TempFile => "Temporary file",
            FileType::DsStore => ".DS_Store",
            FileType::Unknown => "Unknown",
        }
    }

    /// Check if this type is safe to delete without user confirmation
    pub fn is_safe_to_delete(&self) -> bool {
        matches!(
            self,
            FileType::NodeModules
                | FileType::PythonVenv
                | FileType::PythonCache
                | FileType::RustTarget
                | FileType::GoVendor
                | FileType::GradleBuild
                | FileType::MavenTarget
                | FileType::ElixirBuild
                | FileType::ElixirDeps
                | FileType::HaskellStackWork
                | FileType::DsStore
                | FileType::TempFile
        )
    }

    /// Detect file type from path
    pub fn detect(path: &Path) -> Self {
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        match name {
            "node_modules" => FileType::NodeModules,
            ".venv" | "venv" | ".virtualenv" | "virtualenv" => FileType::PythonVenv,
            "__pycache__" | ".pytest_cache" | ".mypy_cache" => FileType::PythonCache,
            "target" if has_cargo_toml(path) => FileType::RustTarget,
            "target" if has_pom_xml(path) => FileType::MavenTarget,
            "vendor" if has_go_mod(path) => FileType::GoVendor,
            "vendor" if has_gemfile(path) => FileType::RubyVendor,
            "vendor" if has_composer(path) => FileType::PhpVendor,
            ".gradle" => FileType::GradleBuild,
            "build" if has_build_gradle(path) => FileType::GradleBuild,
            "bin" if has_csproj(path) => FileType::DotNetBin,
            "obj" if has_csproj(path) => FileType::DotNetObj,
            "_build" if has_mix_exs(path) => FileType::ElixirBuild,
            "deps" if has_mix_exs(path) => FileType::ElixirDeps,
            ".stack-work" => FileType::HaskellStackWork,
            ".DS_Store" => FileType::DsStore,
            _ => {
                if name.ends_with(".log") {
                    FileType::LogFile
                } else if name.ends_with(".tmp") || name.ends_with(".temp") {
                    FileType::TempFile
                } else {
                    FileType::Unknown
                }
            }
        }
    }
}

fn has_cargo_toml(path: &Path) -> bool {
    path.parent()
        .map(|p| p.join("Cargo.toml").exists())
        .unwrap_or(false)
}

fn has_pom_xml(path: &Path) -> bool {
    path.parent()
        .map(|p| p.join("pom.xml").exists())
        .unwrap_or(false)
}

fn has_go_mod(path: &Path) -> bool {
    path.parent()
        .map(|p| p.join("go.mod").exists())
        .unwrap_or(false)
}

fn has_gemfile(path: &Path) -> bool {
    path.parent()
        .map(|p| p.join("Gemfile").exists())
        .unwrap_or(false)
}

fn has_composer(path: &Path) -> bool {
    path.parent()
        .map(|p| p.join("composer.json").exists())
        .unwrap_or(false)
}

fn has_build_gradle(path: &Path) -> bool {
    path.parent()
        .map(|p| p.join("build.gradle").exists() || p.join("build.gradle.kts").exists())
        .unwrap_or(false)
}

fn has_csproj(path: &Path) -> bool {
    if let Some(parent) = path.parent() {
        if let Ok(entries) = std::fs::read_dir(parent) {
            for entry in entries.flatten() {
                if let Some(name) = entry.path().extension() {
                    if name == "csproj" || name == "fsproj" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn has_mix_exs(path: &Path) -> bool {
    path.parent()
        .map(|p| p.join("mix.exs").exists())
        .unwrap_or(false)
}

/// Patterns for dev artifacts to scan
pub const DEV_ARTIFACT_PATTERNS: &[(&str, FileType)] = &[
    ("node_modules", FileType::NodeModules),
    (".venv", FileType::PythonVenv),
    ("venv", FileType::PythonVenv),
    ("__pycache__", FileType::PythonCache),
    ("target", FileType::RustTarget),
    ("vendor", FileType::GoVendor),
    (".gradle", FileType::GradleBuild),
    ("build", FileType::GradleBuild),
    (".stack-work", FileType::HaskellStackWork),
    ("_build", FileType::ElixirBuild),
    ("deps", FileType::ElixirDeps),
    ("bin", FileType::DotNetBin),
    ("obj", FileType::DotNetObj),
];
