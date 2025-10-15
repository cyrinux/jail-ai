use crate::image;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JailConfig {
    /// Name of the jail
    pub name: String,

    /// Backend type (podman)
    pub backend: BackendType,

    /// Base image or directory for the jail
    pub base_image: String,

    /// Directories to bind mount into the jail
    pub bind_mounts: Vec<BindMount>,

    /// Environment variables to set
    pub environment: Vec<(String, String)>,

    /// Network access settings
    pub network: NetworkConfig,

    /// Port mappings from host to container
    #[serde(default)]
    pub port_mappings: Vec<PortMapping>,

    /// Resource limits
    pub limits: ResourceLimits,

    /// Upgrade: rebuild outdated layers and recreate container
    #[serde(default)]
    pub upgrade: bool,

    /// Force specific layers (comma-separated, e.g., "base,rust,python")
    #[serde(default)]
    pub force_layers: Vec<String>,

    /// Use layered images (auto-detect project type and build on-demand)
    #[serde(default = "default_true")]
    pub use_layered_images: bool,

    /// Use isolated project-specific images (workspace hash) instead of shared layer-based images
    #[serde(default)]
    pub isolated: bool,

    /// Show verbose output (e.g., image build logs)
    #[serde(default)]
    pub verbose: bool,

    /// Directories to create in container before mounting (for worktrees)
    #[serde(default)]
    pub pre_create_dirs: Vec<PathBuf>,
    /// Skip nix layer (by default, nix takes precedence over other language layers)
    #[serde(default)]
    pub no_nix: bool,

    /// Enable eBPF-based host blocking (blocks connections to host IPs)
    #[serde(default = "default_true")]
    pub block_host: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BackendType {
    Podman,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindMount {
    pub source: PathBuf,
    pub target: PathBuf,
    pub readonly: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Enable network access (if false, uses --network=none for complete isolation)
    pub enabled: bool,
    /// Use private networking (slirp4netns) for secure, isolated network access
    /// When true with enabled=true: provides internet access without exposing host services
    /// Port forwarding works correctly with private networking for OAuth callbacks
    pub private: bool,
    /// Use host networking (--network=host) for OAuth authentication
    /// When true: container shares host's network namespace (less secure, use only for auth)
    #[serde(default)]
    pub host: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub host_port: u16,
    pub container_port: u16,
    /// Protocol (tcp or udp)
    #[serde(default = "default_tcp")]
    pub protocol: String,
}

fn default_tcp() -> String {
    "tcp".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub memory_mb: Option<u64>,
    pub cpu_quota: Option<u32>,
}

impl BackendType {
    /// Check if this backend is available on the system
    pub fn is_available(&self) -> bool {
        std::process::Command::new("podman")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Get all available backends on the system
    pub fn all_available() -> Vec<Self> {
        if BackendType::Podman.is_available() {
            vec![BackendType::Podman]
        } else {
            vec![]
        }
    }

    /// Always returns Podman (only supported backend)
    pub fn detect() -> Self {
        BackendType::Podman
    }
}

impl Default for JailConfig {
    fn default() -> Self {
        Self {
            name: String::from("ai-agent"),
            backend: BackendType::detect(),
            base_image: String::from(image::DEFAULT_IMAGE_NAME),
            bind_mounts: Vec::new(),
            environment: Vec::new(),
            network: NetworkConfig {
                enabled: true,
                private: true,
                host: false,
            },
            port_mappings: Vec::new(),
            limits: ResourceLimits {
                memory_mb: None,
                cpu_quota: None,
            },
            upgrade: false,
            force_layers: Vec::new(),
            use_layered_images: true,
            isolated: false,
            verbose: false,
            pre_create_dirs: Vec::new(),
            no_nix: false,
            block_host: true,
        }
    }
}
