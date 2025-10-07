use crate::image;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JailConfig {
    /// Name of the jail
    pub name: String,

    /// Backend type (systemd-nspawn or podman)
    pub backend: BackendType,

    /// Base image or directory for the jail
    pub base_image: String,

    /// Directories to bind mount into the jail
    pub bind_mounts: Vec<BindMount>,

    /// Environment variables to set
    pub environment: Vec<(String, String)>,

    /// Network access settings
    pub network: NetworkConfig,

    /// Resource limits
    pub limits: ResourceLimits,

    /// Force rebuild of the default image
    #[serde(default)]
    pub force_rebuild: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BackendType {
    SystemdNspawn,
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
    pub enabled: bool,
    pub private: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub memory_mb: Option<u64>,
    pub cpu_quota: Option<u32>,
}

impl BackendType {
    /// Check if this backend is available on the system
    pub fn is_available(&self) -> bool {
        let command = match self {
            BackendType::Podman => "podman",
            BackendType::SystemdNspawn => "systemd-nspawn",
        };

        std::process::Command::new(command)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Get all available backends on the system
    pub fn all_available() -> Vec<Self> {
        vec![BackendType::Podman, BackendType::SystemdNspawn]
            .into_iter()
            .filter(|b| b.is_available())
            .collect()
    }

    /// Detect which backend is available on the system.
    /// Checks in order: podman -> systemd-nspawn
    /// Returns the first available backend, or Podman as fallback.
    pub fn detect() -> Self {
        use tracing::debug;

        // Check backends in order of preference
        for backend in [BackendType::Podman, BackendType::SystemdNspawn] {
            if backend.is_available() {
                debug!("Detected backend: {:?}", backend);
                return backend;
            }
        }

        // Default to Podman if nothing is detected
        debug!("No backend detected, defaulting to podman");
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
                enabled: false,
                private: true,
            },
            limits: ResourceLimits {
                memory_mb: None,
                cpu_quota: None,
            },
            force_rebuild: false,
        }
    }
}
