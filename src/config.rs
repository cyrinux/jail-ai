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

impl Default for JailConfig {
    fn default() -> Self {
        Self {
            name: String::from("ai-agent"),
            backend: BackendType::Podman,
            base_image: String::from("localhost/jail-ai-env:latest"),
            bind_mounts: Vec::new(),
            environment: Vec::new(),
            network: NetworkConfig {
                enabled: false,
                private: true,
            },
            limits: ResourceLimits {
                memory_mb: Some(512),
                cpu_quota: Some(50),
            },
        }
    }
}
