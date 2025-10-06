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

    async fn remove(&self, name: &str, _remove_volume: bool) -> Result<()> {
        info!("Removing systemd-nspawn jail: {}", name);

        // Try to terminate if running (ignore errors if not running)
        let mut cmd = Command::new("machinectl");
        cmd.arg("terminate").arg(name);
        let _ = run_command(&mut cmd).await;

        // Remove machine directory
        let machine_path = self.get_machine_path(name);
        tokio::fs::remove_dir_all(&machine_path).await?;

        info!("Jail {} removed", name);
        Ok(())
    }

    async fn exec(&self, name: &str, command: &[String], interactive: bool) -> Result<String> {
        debug!(
            "Executing command in jail {}: {:?} (interactive: {})",
            name, command, interactive
        );

        let mut cmd = Command::new("systemd-run");
        cmd.arg("--machine").arg(name).arg("--wait");

        if interactive {
            // Interactive mode: use PTY for terminal support
            cmd.arg("--pty");
        } else {
            // Non-interactive mode: use pipe for output capture
            cmd.arg("--pipe");
        }

        for arg in command {
            cmd.arg(arg);
        }

        if interactive {
            // Interactive mode: inherit stdio for direct user interaction
            use std::process::Stdio;
            cmd.stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());

            let status = cmd.status().await.map_err(|e| {
                JailError::Backend(format!("Failed to execute interactive command: {}", e))
            })?;

            if !status.success() {
                return Err(JailError::ExecutionFailed(format!(
                    "Interactive command failed with status: {}",
                    status
                )));
            }

            Ok(String::new()) // No output to capture in interactive mode
        } else {
            // Non-interactive mode: capture output
            let output = run_command(&mut cmd).await?;
            debug!("Command output: {}", output);
            Ok(output)
        }
    }

    async fn exists(&self, name: &str) -> Result<bool> {
        let machine_path = self.get_machine_path(name);
        Ok(machine_path.exists())
    }

    async fn list_all(&self) -> Result<Vec<String>> {
        debug!("Listing all jail-ai machines");

        // List all directories in /var/lib/machines that start with "jail-"
        let mut jails = Vec::new();

        if let Ok(mut entries) = tokio::fs::read_dir(&self.machines_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(file_name) = entry.file_name().into_string() {
                    if file_name.starts_with("jail-") {
                        jails.push(file_name);
                    }
                }
            }
        }

        debug!("Found {} jail-ai machines", jails.len());
        Ok(jails)
    }

    async fn inspect(&self, name: &str) -> Result<JailConfig> {
        debug!("Inspecting systemd-nspawn jail: {}", name);

        if !self.exists(name).await? {
            return Err(JailError::NotFound(format!("Jail '{}' not found", name)));
        }

        // For systemd-nspawn, we cannot easily retrieve the full configuration
        // from a running/stopped machine. We return a minimal config with the name
        // and detected backend type. Users should maintain their own config files
        // if they need to preserve full configurations.

        // Try to read machine metadata if available
        let machine_path = self.get_machine_path(name);

        // Return minimal config - systemd-nspawn doesn't store runtime config in an easily parseable format
        Ok(JailConfig {
            name: name.to_string(),
            backend: crate::config::BackendType::SystemdNspawn,
            base_image: format!("<machine at {}>", machine_path.display()),
            bind_mounts: Vec::new(),
            environment: Vec::new(),
            network: crate::config::NetworkConfig {
                enabled: false,
                private: true,
            },
            limits: crate::config::ResourceLimits {
                memory_mb: None,
                cpu_quota: None,
            },
        })
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
