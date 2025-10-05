use super::{run_command, JailBackend};
use crate::config::JailConfig;
use crate::error::{JailError, Result};
use async_trait::async_trait;
use tokio::process::Command;
use tracing::{debug, info};

pub struct DockerBackend;

impl DockerBackend {
    pub fn new() -> Self {
        Self
    }

    async fn image_exists(&self, image: &str) -> Result<bool> {
        let mut cmd = Command::new("docker");
        cmd.arg("image").arg("inspect").arg(image);

        match cmd.output().await {
            Ok(output) => Ok(output.status.success()),
            Err(_) => Ok(false),
        }
    }

    fn build_run_args(&self, config: &JailConfig) -> Vec<String> {
        let mut args = vec![
            "run".to_string(),
            "-d".to_string(),
            "--name".to_string(),
            config.name.clone(),
        ];

        // User mapping - Docker doesn't support --userns=keep-id
        // Use --user with host UID:GID instead
        // Get UID and GID from environment or use current user
        let uid = std::env::var("UID")
            .or_else(|_| {
                // Fallback: get from id command
                std::process::Command::new("id")
                    .arg("-u")
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| s.trim().to_string())
                    .ok_or_else(|| std::io::Error::other("Failed to get UID"))
            })
            .unwrap_or_else(|_| "1000".to_string());

        let gid = std::env::var("GID")
            .or_else(|_| {
                // Fallback: get from id command
                std::process::Command::new("id")
                    .arg("-g")
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| s.trim().to_string())
                    .ok_or_else(|| std::io::Error::other("Failed to get GID"))
            })
            .unwrap_or_else(|_| "100".to_string());

        args.push("--user".to_string());
        args.push(format!("{}:{}", uid, gid));

        // Network settings
        if !config.network.enabled {
            args.push("--network=none".to_string());
        } else if config.network.private {
            // Docker uses "bridge" for private network by default
            args.push("--network=bridge".to_string());
        }

        // Bind mounts
        for mount in &config.bind_mounts {
            let bind_arg = if mount.readonly {
                format!("{}:{}:ro", mount.source.display(), mount.target.display())
            } else {
                format!("{}:{}", mount.source.display(), mount.target.display())
            };
            args.push("-v".to_string());
            args.push(bind_arg);
        }

        // Environment variables
        for (key, value) in &config.environment {
            args.push("-e".to_string());
            args.push(format!("{}={}", key, value));
        }

        // Resource limits
        if let Some(memory_mb) = config.limits.memory_mb {
            args.push("-m".to_string());
            args.push(format!("{}m", memory_mb));
        }
        if let Some(cpu_quota) = config.limits.cpu_quota {
            args.push("--cpus".to_string());
            args.push(format!("{}", cpu_quota as f64 / 100.0));
        }

        // Base image
        args.push(config.base_image.clone());

        // Keep container running
        args.push("sleep".to_string());
        args.push("infinity".to_string());

        args
    }
}

impl Default for DockerBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl JailBackend for DockerBackend {
    async fn create(&self, config: &JailConfig) -> Result<()> {
        info!("Creating docker jail: {}", config.name);

        if self.exists(&config.name).await? {
            return Err(JailError::AlreadyExists(config.name.clone()));
        }

        // Check if image exists locally, if not pull it
        let image_exists = self.image_exists(&config.base_image).await?;

        if !image_exists {
            debug!("Image {} not found locally, pulling...", config.base_image);
            let mut pull_cmd = Command::new("docker");
            pull_cmd.arg("pull").arg(&config.base_image);

            run_command(&mut pull_cmd)
                .await
                .map_err(|e| JailError::Backend(format!("Failed to pull image: {}", e)))?;
        } else {
            debug!("Using local image: {}", config.base_image);
        }

        // Create and start the container
        let args = self.build_run_args(config);
        let mut cmd = Command::new("docker");
        cmd.args(&args);

        debug!("Creating container with args: {:?}", args);
        run_command(&mut cmd).await?;

        info!("Jail {} created successfully", config.name);
        Ok(())
    }

    async fn start(&self, name: &str) -> Result<()> {
        info!("Starting docker jail: {}", name);

        let mut cmd = Command::new("docker");
        cmd.arg("start").arg(name);

        run_command(&mut cmd).await?;

        info!("Jail {} started", name);
        Ok(())
    }

    async fn stop(&self, name: &str) -> Result<()> {
        info!("Stopping docker jail: {}", name);

        let mut cmd = Command::new("docker");
        cmd.arg("stop").arg(name);

        run_command(&mut cmd).await?;

        info!("Jail {} stopped", name);
        Ok(())
    }

    async fn remove(&self, name: &str) -> Result<()> {
        info!("Removing docker jail: {}", name);

        // Stop if running
        let _ = self.stop(name).await;

        // Remove container
        let mut cmd = Command::new("docker");
        cmd.arg("rm").arg("-f").arg(name);

        run_command(&mut cmd).await?;

        info!("Jail {} removed", name);
        Ok(())
    }

    async fn exec(&self, name: &str, command: &[String], interactive: bool) -> Result<String> {
        debug!(
            "Executing command in jail {}: {:?} (interactive: {})",
            name, command, interactive
        );

        let mut cmd = Command::new("docker");
        cmd.arg("exec");

        if interactive {
            cmd.arg("-it");
        }

        cmd.arg(name);

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
        let mut cmd = Command::new("docker");
        cmd.arg("ps")
            .arg("-a")
            .arg("--filter")
            .arg(format!("name=^{}$", name))
            .arg("--format")
            .arg("{{.Names}}");

        match run_command(&mut cmd).await {
            Ok(output) => Ok(output.trim() == name),
            Err(_) => Ok(false),
        }
    }

    async fn list_all(&self) -> Result<Vec<String>> {
        debug!("Listing all jail-ai containers");

        let mut cmd = Command::new("docker");
        cmd.arg("ps").arg("-a").arg("--format").arg("{{.Names}}");

        let output = run_command(&mut cmd).await?;

        // Filter containers that start with "jail-"
        let jails: Vec<String> = output
            .lines()
            .filter(|line| line.starts_with("jail-"))
            .map(|line| line.to_string())
            .collect();

        debug!("Found {} jail-ai containers", jails.len());
        Ok(jails)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_run_args() {
        // Set env vars for test
        unsafe {
            std::env::set_var("UID", "1000");
            std::env::set_var("GID", "100");
        }

        let backend = DockerBackend::new();
        let config = JailConfig {
            name: "test-jail".to_string(),
            backend: crate::config::BackendType::Docker,
            base_image: "alpine:latest".to_string(),
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

        let args = backend.build_run_args(&config);

        assert!(args.contains(&"run".to_string()));
        assert!(args.contains(&"--name".to_string()));
        assert!(args.contains(&"test-jail".to_string()));
        assert!(args.contains(&"--user".to_string()));
        assert!(args.contains(&"-m".to_string()));
        assert!(args.contains(&"512m".to_string()));
        assert!(args.contains(&"-e".to_string()));
        assert!(args.contains(&"TEST=value".to_string()));
    }

    #[test]
    fn test_list_all_filters_jail_prefix() {
        // This is a unit test that verifies the filtering logic would work
        let names = vec![
            "jail-test-abc123",
            "jail-project-def456",
            "other-container",
            "jail-another-xyz789",
            "my-container",
        ];

        let filtered: Vec<String> = names
            .into_iter()
            .filter(|name| name.starts_with("jail-"))
            .map(|s| s.to_string())
            .collect();

        assert_eq!(filtered.len(), 3);
        assert!(filtered.contains(&"jail-test-abc123".to_string()));
        assert!(filtered.contains(&"jail-project-def456".to_string()));
        assert!(filtered.contains(&"jail-another-xyz789".to_string()));
    }
}
