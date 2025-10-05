use super::{run_command, JailBackend};
use crate::config::JailConfig;
use crate::error::{JailError, Result};
use async_trait::async_trait;
use tokio::process::Command;
use tracing::{debug, info};

pub struct PodmanBackend;

impl PodmanBackend {
    pub fn new() -> Self {
        Self
    }

    async fn image_exists(&self, image: &str) -> Result<bool> {
        let mut cmd = Command::new("podman");
        cmd.arg("image").arg("exists").arg(image);

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

        // Network settings
        if !config.network.enabled {
            args.push("--network=none".to_string());
        } else if config.network.private {
            args.push("--network=private".to_string());
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

impl Default for PodmanBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl JailBackend for PodmanBackend {
    async fn create(&self, config: &JailConfig) -> Result<()> {
        info!("Creating podman jail: {}", config.name);

        if self.exists(&config.name).await? {
            return Err(JailError::AlreadyExists(config.name.clone()));
        }

        // Check if image exists locally, if not pull it
        let image_exists = self.image_exists(&config.base_image).await?;

        if !image_exists {
            debug!("Image {} not found locally, pulling...", config.base_image);
            let mut pull_cmd = Command::new("podman");
            pull_cmd.arg("pull").arg(&config.base_image);

            run_command(&mut pull_cmd).await.map_err(|e| {
                JailError::Backend(format!("Failed to pull image: {}", e))
            })?;
        } else {
            debug!("Using local image: {}", config.base_image);
        }

        // Create and start the container
        let args = self.build_run_args(config);
        let mut cmd = Command::new("podman");
        cmd.args(&args);

        debug!("Creating container with args: {:?}", args);
        run_command(&mut cmd).await?;

        info!("Jail {} created successfully", config.name);
        Ok(())
    }

    async fn start(&self, name: &str) -> Result<()> {
        info!("Starting podman jail: {}", name);

        let mut cmd = Command::new("podman");
        cmd.arg("start").arg(name);

        run_command(&mut cmd).await?;

        info!("Jail {} started", name);
        Ok(())
    }

    async fn stop(&self, name: &str) -> Result<()> {
        info!("Stopping podman jail: {}", name);

        let mut cmd = Command::new("podman");
        cmd.arg("stop").arg(name);

        run_command(&mut cmd).await?;

        info!("Jail {} stopped", name);
        Ok(())
    }

    async fn remove(&self, name: &str) -> Result<()> {
        info!("Removing podman jail: {}", name);

        // Stop if running
        let _ = self.stop(name).await;

        // Remove container
        let mut cmd = Command::new("podman");
        cmd.arg("rm").arg("-f").arg(name);

        run_command(&mut cmd).await?;

        info!("Jail {} removed", name);
        Ok(())
    }

    async fn exec(&self, name: &str, command: &[String]) -> Result<String> {
        debug!("Executing command in jail {}: {:?}", name, command);

        let mut cmd = Command::new("podman");
        cmd.arg("exec").arg(name);

        for arg in command {
            cmd.arg(arg);
        }

        let output = run_command(&mut cmd).await?;
        debug!("Command output: {}", output);

        Ok(output)
    }

    async fn exists(&self, name: &str) -> Result<bool> {
        let mut cmd = Command::new("podman");
        cmd.arg("ps").arg("-a").arg("--filter").arg(format!("name={}", name)).arg("--format").arg("{{.Names}}");

        match run_command(&mut cmd).await {
            Ok(output) => Ok(output.trim() == name),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_run_args() {
        let backend = PodmanBackend::new();
        let config = JailConfig {
            name: "test-jail".to_string(),
            backend: crate::config::BackendType::Podman,
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
        assert!(args.contains(&"-m".to_string()));
        assert!(args.contains(&"512m".to_string()));
        assert!(args.contains(&"-e".to_string()));
        assert!(args.contains(&"TEST=value".to_string()));
    }
}
