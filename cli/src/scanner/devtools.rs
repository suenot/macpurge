//! Dev tools scanner for Xcode, Android Studio, Docker, VMs

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

use super::{calculate_dir_size, calculate_dir_size_on_disk};

/// Types of development tools
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevToolType {
    Xcode,
    AndroidStudio,
    Docker,
    VirtualMachines,
}

impl DevToolType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Xcode => "Xcode",
            Self::AndroidStudio => "Android Studio",
            Self::Docker => "Docker",
            Self::VirtualMachines => "Virtual Machines",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![Self::Xcode, Self::AndroidStudio, Self::Docker, Self::VirtualMachines]
    }
}

/// A cache path for a dev tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevToolCachePath {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub description: String,
    pub category: String,
    pub safe_to_clean: bool,
}

impl DevToolCachePath {
    pub fn size_human(&self) -> String {
        crate::utils::format_size(self.size_bytes)
    }
}

/// Information about a specific dev tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevToolInfo {
    pub tool: DevToolType,
    pub is_installed: bool,
    pub cache_paths: Vec<DevToolCachePath>,
    pub total_size: u64,
}

impl DevToolInfo {
    pub fn total_size_human(&self) -> String {
        crate::utils::format_size(self.total_size)
    }
}

/// Result of scanning all dev tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevToolScanResult {
    pub tools: Vec<DevToolInfo>,
    pub total_size: u64,
    pub scan_duration_ms: u64,
}

impl DevToolScanResult {
    pub fn total_size_human(&self) -> String {
        crate::utils::format_size(self.total_size)
    }
}

/// Scanner for development tool caches
pub struct DevToolScanner {
    home_dir: PathBuf,
}

impl DevToolScanner {
    pub fn new() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        Self { home_dir }
    }

    /// Scan all dev tools
    pub fn scan_all(&self) -> Result<DevToolScanResult> {
        let start = std::time::Instant::now();
        let mut tools = Vec::new();

        for tool_type in DevToolType::all() {
            if let Ok(info) = self.scan_tool(tool_type) {
                tools.push(info);
            }
        }

        let total_size = tools.iter().map(|t| t.total_size).sum();

        Ok(DevToolScanResult {
            tools,
            total_size,
            scan_duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Scan a specific dev tool
    pub fn scan_tool(&self, tool: DevToolType) -> Result<DevToolInfo> {
        match tool {
            DevToolType::Xcode => self.scan_xcode(),
            DevToolType::AndroidStudio => self.scan_android(),
            DevToolType::Docker => self.scan_docker(),
            DevToolType::VirtualMachines => self.scan_vms(),
        }
    }

    fn scan_xcode(&self) -> Result<DevToolInfo> {
        let is_installed = which::which("xcodebuild").is_ok()
            || self.home_dir.join("Library/Developer/Xcode").exists();

        let mut cache_paths = Vec::new();

        // DerivedData — build artifacts, often 10-50GB
        let derived_data = self.home_dir.join("Library/Developer/Xcode/DerivedData");
        self.add_path_if_exists(&mut cache_paths, &derived_data, "Xcode DerivedData (build artifacts)", "build", true);

        // Archives — old app archives
        let archives = self.home_dir.join("Library/Developer/Xcode/Archives");
        self.add_path_if_exists(&mut cache_paths, &archives, "Xcode Archives", "archives", true);

        // iOS DeviceSupport — debug symbols for connected devices
        let ios_support = self.home_dir.join("Library/Developer/Xcode/iOS DeviceSupport");
        self.add_path_if_exists(&mut cache_paths, &ios_support, "iOS DeviceSupport (debug symbols)", "device_support", true);

        // watchOS DeviceSupport
        let watch_support = self.home_dir.join("Library/Developer/Xcode/watchOS DeviceSupport");
        self.add_path_if_exists(&mut cache_paths, &watch_support, "watchOS DeviceSupport", "device_support", true);

        // CoreSimulator Devices — simulator data
        let sim_devices = self.home_dir.join("Library/Developer/CoreSimulator/Devices");
        self.add_path_if_exists(&mut cache_paths, &sim_devices, "Simulator Devices", "simulators", true);

        // CoreSimulator Caches
        let sim_caches = self.home_dir.join("Library/Developer/CoreSimulator/Caches");
        self.add_path_if_exists(&mut cache_paths, &sim_caches, "Simulator Caches", "simulators", true);

        // Xcode caches
        let xcode_cache = self.home_dir.join("Library/Caches/com.apple.dt.Xcode");
        self.add_path_if_exists(&mut cache_paths, &xcode_cache, "Xcode cache", "cache", true);

        let total_size = cache_paths.iter().map(|p| p.size_bytes).sum();

        Ok(DevToolInfo {
            tool: DevToolType::Xcode,
            is_installed,
            cache_paths,
            total_size,
        })
    }

    fn scan_android(&self) -> Result<DevToolInfo> {
        let android_sdk = self.home_dir.join("Library/Android/sdk");
        let is_installed = which::which("adb").is_ok() || android_sdk.exists();

        let mut cache_paths = Vec::new();

        // Android SDK
        self.add_path_if_exists(&mut cache_paths, &android_sdk, "Android SDK", "sdk", false);

        // AVD (Android Virtual Devices)
        let avd = self.home_dir.join(".android/avd");
        self.add_path_if_exists(&mut cache_paths, &avd, "Android Virtual Devices (AVD)", "avd", true);

        // Gradle caches
        let gradle_caches = self.home_dir.join(".gradle/caches");
        self.add_path_if_exists(&mut cache_paths, &gradle_caches, "Gradle caches", "gradle", true);

        // Gradle wrapper distributions
        let gradle_dists = self.home_dir.join(".gradle/wrapper/dists");
        self.add_path_if_exists(&mut cache_paths, &gradle_dists, "Gradle wrapper distributions", "gradle", true);

        let total_size = cache_paths.iter().map(|p| p.size_bytes).sum();

        Ok(DevToolInfo {
            tool: DevToolType::AndroidStudio,
            is_installed,
            cache_paths,
            total_size,
        })
    }

    fn scan_docker(&self) -> Result<DevToolInfo> {
        let is_installed = which::which("docker").is_ok();
        let mut cache_paths = Vec::new();

        if is_installed {
            // Try to get Docker system df info
            if let Ok(output) = Command::new("docker")
                .args(["system", "df", "--format", "{{json .}}"])
                .output()
            {
                if output.status.success() {
                    if let Ok(stdout) = String::from_utf8(output.stdout) {
                        for line in stdout.lines() {
                            if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                                let type_name = val.get("Type").and_then(|v| v.as_str()).unwrap_or("");
                                let size_str = val.get("Size").and_then(|v| v.as_str()).unwrap_or("0B");
                                let reclaimable = val.get("Reclaimable").and_then(|v| v.as_str()).unwrap_or("0B");

                                let size = parse_docker_size(size_str);
                                let reclaim = parse_docker_size(reclaimable);

                                if size > 0 {
                                    cache_paths.push(DevToolCachePath {
                                        path: PathBuf::from(format!("docker://{}", type_name.to_lowercase())),
                                        size_bytes: size,
                                        description: format!("Docker {} (reclaimable: {})", type_name, crate::utils::format_size(reclaim)),
                                        category: "docker".to_string(),
                                        safe_to_clean: false,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // Docker Desktop VM disk (on macOS)
        // NOTE: The VM disk *contains* all images/containers/volumes/cache,
        // so we must not sum them — that would be double-counting.
        let mut vm_disk_size = 0u64;
        let docker_vm = self.home_dir.join("Library/Containers/com.docker.docker/Data/vms");
        if docker_vm.exists() {
            // Use on-disk size: Docker.raw is a sparse file — logical size can be
            // 64-256 GB while actual disk usage is much smaller (e.g. 7 GB).
            let (size, _) = calculate_dir_size_on_disk(&docker_vm);
            if size > 0 {
                vm_disk_size = size;
                cache_paths.push(DevToolCachePath {
                    path: docker_vm,
                    size_bytes: size,
                    description: "Docker Desktop VM disk (contains all Docker data)".to_string(),
                    category: "docker_vm".to_string(),
                    safe_to_clean: false,
                });
            }
        }

        // total_size: use VM disk size if present (it encompasses everything),
        // otherwise fall back to sum of docker system df entries (e.g. on Linux).
        let docker_df_total: u64 = cache_paths.iter()
            .filter(|p| p.category == "docker")
            .map(|p| p.size_bytes)
            .sum();
        let total_size = if vm_disk_size > 0 { vm_disk_size } else { docker_df_total };

        Ok(DevToolInfo {
            tool: DevToolType::Docker,
            is_installed,
            cache_paths,
            total_size,
        })
    }

    fn scan_vms(&self) -> Result<DevToolInfo> {
        let mut cache_paths = Vec::new();
        let mut is_installed = false;

        // Parallels
        let parallels = self.home_dir.join("Parallels");
        if parallels.exists() {
            is_installed = true;
            self.add_path_if_exists(&mut cache_paths, &parallels, "Parallels VMs", "parallels", false);
        }

        // UTM
        let utm = self.home_dir.join("Library/Containers/com.utmapp.UTM/Data/Documents");
        if utm.exists() {
            is_installed = true;
            self.add_path_if_exists(&mut cache_paths, &utm, "UTM VMs", "utm", false);
        }

        // VMware Fusion
        let vmware = self.home_dir.join("Virtual Machines.localized");
        if vmware.exists() {
            is_installed = true;
            self.add_path_if_exists(&mut cache_paths, &vmware, "VMware Fusion VMs", "vmware", false);
        }

        // VirtualBox
        let vbox = self.home_dir.join("VirtualBox VMs");
        if vbox.exists() {
            is_installed = true;
            self.add_path_if_exists(&mut cache_paths, &vbox, "VirtualBox VMs", "virtualbox", false);
        }

        let total_size = cache_paths.iter().map(|p| p.size_bytes).sum();

        Ok(DevToolInfo {
            tool: DevToolType::VirtualMachines,
            is_installed,
            cache_paths,
            total_size,
        })
    }

    fn add_path_if_exists(
        &self,
        paths: &mut Vec<DevToolCachePath>,
        path: &PathBuf,
        description: &str,
        category: &str,
        safe_to_clean: bool,
    ) {
        if path.exists() {
            let size = if path.is_dir() {
                let (s, _) = calculate_dir_size(path);
                s
            } else {
                path.metadata().map(|m| m.len()).unwrap_or(0)
            };

            if size > 0 {
                paths.push(DevToolCachePath {
                    path: path.clone(),
                    size_bytes: size,
                    description: description.to_string(),
                    category: category.to_string(),
                    safe_to_clean,
                });
            }
        }
    }
}

/// Parse Docker size strings like "1.5GB", "100MB", "2.3kB" (public alias)
pub fn parse_docker_size_pub(s: &str) -> u64 {
    parse_docker_size(s)
}

/// Parse Docker size strings like "1.5GB", "100MB", "2.3kB"
fn parse_docker_size(s: &str) -> u64 {
    // Remove parenthesized percentage like "(50%)" at the end
    let s = s.split('(').next().unwrap_or(s).trim();

    let s_lower = s.to_lowercase();
    let (num_str, multiplier) = if s_lower.ends_with("tb") {
        (&s[..s.len()-2], 1024u64 * 1024 * 1024 * 1024)
    } else if s_lower.ends_with("gb") {
        (&s[..s.len()-2], 1024u64 * 1024 * 1024)
    } else if s_lower.ends_with("mb") {
        (&s[..s.len()-2], 1024u64 * 1024)
    } else if s_lower.ends_with("kb") {
        (&s[..s.len()-2], 1024u64)
    } else if s_lower.ends_with('b') {
        (&s[..s.len()-1], 1u64)
    } else {
        (s, 1u64)
    };

    num_str.trim().parse::<f64>().map(|n| (n * multiplier as f64) as u64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_devtool_type_all() {
        assert_eq!(DevToolType::all().len(), 4);
    }

    #[test]
    fn test_parse_docker_size() {
        assert_eq!(parse_docker_size("0B"), 0);
        assert_eq!(parse_docker_size("1024B"), 1024);
        assert_eq!(parse_docker_size("1.5GB"), (1.5 * 1024.0 * 1024.0 * 1024.0) as u64);
        assert_eq!(parse_docker_size("100MB"), 100 * 1024 * 1024);
        assert_eq!(parse_docker_size("500MB (30%)"), 500 * 1024 * 1024);
    }

    #[test]
    fn test_scanner_creation() {
        let scanner = DevToolScanner::new();
        let result = scanner.scan_all();
        assert!(result.is_ok());
    }
}
