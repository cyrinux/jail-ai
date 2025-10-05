use super::{run_command, JailBackend};
use crate::config::JailConfig;
use crate::error::{JailError, Result};
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::process::Command;
use tracing::{debug, info};

pub struct SystemdNspawnBackend {
    machines_dir: PathBuf,
}

impl SystemdNspawnBackend {
    pub fn new() -> Self {
        Self {
            machines_dir: PathBuf::from("/var/lib/machines"),
        }
    }

    fn get_machine_path(&self, name: &str) -> PathBuf {
        self.machines_dir.join(name)
    }

    #[allow(dead_code)]
    fn build_nspawn_command(&self, config: &JailConfig) -> Command {
        let mut cmd = Command::new("systemd-nspawn");

        // Set machine name
        cmd.arg("--machine").arg(&config.name);

        // Set directory
        cmd.arg("--directory")
            .arg(self.get_machine_path(&config.name));

        // Network settings
        if config.network.enabled {
            if config.network.private {
                cmd.arg("--network-veth");
            } else {
                cmd.arg("--network-bridge=br0");
            }
        } else {
            cmd.arg("--private-network");
        }

        // Bind mounts
        for mount in &config.bind_mounts {
            let bind_arg = if mount.readonly {
                format!("{}:{}:ro", mount.source.display(), mount.target.display())
            } else {
                format!("{}:{}", mount.source.display(), mount.target.display())
            };
            cmd.arg("--bind").arg(bind_arg);
        }

        // Environment variables
        for (key, value) in &config.environment {
            cmd.arg("--setenv").arg(format!("{}={}", key, value));
        }

        // Resource limits
        if let Some(memory_mb) = config.limits.memory_mb {
            cmd.arg("--property")
                .arg(format!("MemoryMax={}M", memory_mb));
        }
        if let Some(cpu_quota) = config.limits.cpu_quota {
            cmd.arg("--property")
                .arg(format!("CPUQuota={}%", cpu_quota));
        }

        cmd
    }
}

impl Default for SystemdNspawnBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl JailBackend for SystemdNspawnBackend {
    async fn create(&self, config: &JailConfig) -> Result<()> {
        info!("Creating systemd-nspawn jail: {}", config.name);

        if self.exists(&config.name).await? {
            return Err(JailError::AlreadyExists(config.name.clone()));
        }

        let machine_path = self.get_machine_path(&config.name);

        // Create machine directory
        tokio::fs::create_dir_all(&machine_path).await?;
        debug!("Created machine directory: {:?}", machine_path);

        // Bootstrap base system (simplified - in production would use debootstrap or similar)
        let mut cmd = Command::new("debootstrap");
        cmd.arg("--variant=minbase")
            .arg(&config.base_image)
            .arg(&machine_path);

        debug!("Bootstrapping base system with: {:?}", cmd);

        run_command(&mut cmd).await.map_err(|e| {
            JailError::Backend(format!(
                "Failed to bootstrap base system (ensure debootstrap is installed): {}",
                e
            ))
        })?;

        info!("Jail {} created successfully", config.name);
        Ok(())
    }

    async fn start(&self, name: &str) -> Result<()> {
        info!("Starting systemd-nspawn jail: {}", name);

        let mut cmd = Command::new("machinectl");
        cmd.arg("start").arg(name);

        run_command(&mut cmd).await?;

        info!("Jail {} started", name);
        Ok(())
    }

    async fn stop(&self, name: &str) -> Result<()> {
        info!("Stopping systemd-nspawn jail: {}", name);

        let mut cmd = Command::new("machinectl");
        cmd.arg("terminate").arg(name);

        run_command(&mut cmd).await?;

        info!("Jail {} stopped", name);
        Ok(())
    }

    async fn remove(&self, name: &str) -> Result<()> {
        info!("Removing systemd-nspawn jail: {}", name);

        // Stop if running
        let _ = self.stop(name).await;

        // Remove machine directory
        let machine_path = self.get_machine_path(name);
        tokio::fs::remove_dir_all(&machine_path).await?;

        info!("Jail {} removed", name);
        Ok(())
    }

    async fn exec(&self, name: &str, command: &[String]) -> Result<String> {
        debug!("Executing command in jail {}: {:?}", name, command);

        let mut cmd = Command::new("systemd-run");
        cmd.arg("--machine").arg(name).arg("--wait").arg("--pipe");

        for arg in command {
            cmd.arg(arg);
        }

        let output = run_command(&mut cmd).await?;
        debug!("Command output: {}", output);

        Ok(output)
    }

    async fn exists(&self, name: &str) -> Result<bool> {
        let machine_path = self.get_machine_path(name);
        Ok(machine_path.exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_machine_path() {
        let backend = SystemdNspawnBackend::new();
        let path = backend.get_machine_path("test-jail");
        assert_eq!(path, PathBuf::from("/var/lib/machines/test-jail"));
    }

    #[test]
    fn test_build_nspawn_command() {
        let backend = SystemdNspawnBackend::new();
        let config = JailConfig {
            name: "test-jail".to_string(),
            backend: crate::config::BackendType::SystemdNspawn,
            base_image: "debian".to_string(),
            bind_mounts: vec![],
            environment: vec![("TEST".to_string(), "value".to_string())],
            network: crate::config::NetworkConfig {
                enabled: false,
                private: true,
            },
            limits: crate::config::ResourceLimits {
                memory_mb: Some(512),
                cpu_quota: Some(50),
            },
        };

        let cmd = backend.build_nspawn_command(&config);
        let program = cmd.as_std().get_program();
        assert_eq!(program, "systemd-nspawn");
    }
}
