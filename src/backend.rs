use crate::config::JailConfig;
use crate::error::{JailError, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;
use tracing::debug;

#[async_trait]
pub trait JailBackend: Send + Sync {
    /// Create a new jail instance
    async fn create(&self, config: &JailConfig) -> Result<()>;

    /// Remove the jail
    async fn remove(&self, name: &str, remove_volume: bool) -> Result<()>;

    /// Execute a command inside the jail
    async fn exec(&self, name: &str, command: &[String], interactive: bool) -> Result<String>;

    /// Check if jail exists
    async fn exists(&self, name: &str) -> Result<bool>;

    /// List all jail-ai containers (names starting with "jail-")
    async fn list_all(&self) -> Result<Vec<String>>;

    /// Inspect jail and return its configuration
    async fn inspect(&self, name: &str) -> Result<JailConfig>;
}

pub mod podman;
pub mod systemd_nspawn;

/// Create a backend based on the configuration
pub fn create_backend(config: &JailConfig) -> Box<dyn JailBackend> {
    match config.backend {
        crate::config::BackendType::SystemdNspawn => {
            Box::new(systemd_nspawn::SystemdNspawnBackend::new())
        }
        crate::config::BackendType::Podman => Box::new(podman::PodmanBackend::new()),
    }
}

/// Helper to run a command and capture output
async fn run_command(cmd: &mut Command) -> Result<String> {
    debug!("Running command: {:?}", cmd);

    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| JailError::Backend(format!("Failed to execute command: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(JailError::ExecutionFailed(format!(
            "Command failed with status {}: {}",
            output.status, stderr
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
