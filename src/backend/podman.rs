use super::{run_command, JailBackend};
use crate::config::JailConfig;
use crate::error::{JailError, Result};
use crate::image;
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

    /// Get the image currently used by a container
    pub async fn get_container_image(&self, name: &str) -> Result<String> {
        let mut cmd = Command::new("podman");
        cmd.arg("inspect")
            .arg(name)
            .arg("--format")
            .arg("{{.Config.Image}}");

        let output = run_command(&mut cmd).await?;
        Ok(output.trim().to_string())
    }

    fn build_run_args(&self, config: &JailConfig) -> Vec<String> {
        let mut args = vec![
            "run".to_string(),
            "-d".to_string(),
            "--name".to_string(),
            config.name.clone(),
        ];

        args.extend(vec![
            // Preserve user ID mapping from host to avoid permission issues with bind mounts
            "--userns=keep-id".to_string(),
        ]);

        // Persistent volume for /home/agent to preserve data across upgrades
        let home_volume = format!("{}-home", config.name);
        args.push("-v".to_string());
        args.push(format!("{home_volume}:/home/agent"));

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
            args.push(format!("{key}={value}"));
        }

        // Resource limits
        if let Some(memory_mb) = config.limits.memory_mb {
            args.push("-m".to_string());
            args.push(format!("{memory_mb}m"));
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

        // If upgrade is true, stop and remove existing container first
        if config.upgrade && self.exists(&config.name).await? {
            info!(
                "Upgrade enabled: stopping and removing existing container '{}'",
                config.name
            );

            // Stop and remove the existing container (using -f flag to force)
            let mut rm_cmd = Command::new("podman");
            rm_cmd.arg("rm").arg("-f").arg(&config.name);

            if let Err(e) = run_command(&mut rm_cmd).await {
                // Log warning but continue - the container might already be gone
                debug!("Failed to remove existing container (may not exist): {}", e);
            }
        } else if !config.upgrade && self.exists(&config.name).await? {
            return Err(JailError::AlreadyExists(config.name.clone()));
        }

        // Determine which image to use
        let actual_image = if config.base_image == image::DEFAULT_IMAGE_NAME
            && config.use_layered_images
        {
            // Use layered image system with auto-detection
            info!("Using layered image system with auto-detection");

            // Try to find workspace path from bind mounts
            let workspace_path = config
                .bind_mounts
                .iter()
                .find(|m| {
                    m.target
                        .to_str()
                        .map(|s| s.contains("workspace"))
                        .unwrap_or(false)
                })
                .map(|m| m.source.clone())
                .unwrap_or_else(|| {
                    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
                });

            // Try to detect agent from jail name (format: jail-{project}-{hash}-{agent})
            let agent_name = config
                .name
                .rsplit('-')
                .next()
                .and_then(|suffix| match suffix {
                    "claude" | "copilot" | "cursor" | "gemini" | "codex" => Some(suffix),
                    "agent" => {
                        // Handle legacy jail names with format: ...-cursor-agent
                        // Look at the second-to-last segment
                        let parts: Vec<&str> = config.name.rsplitn(3, '-').collect();
                        if parts.len() >= 2 {
                            match parts[1] {
                                "cursor" => Some("cursor"),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    }
                    _ => None,
                });

            debug!(
                "Workspace path: {:?}, Agent: {:?}",
                workspace_path, agent_name
            );

            // Build the appropriate layered image
            crate::image_layers::ensure_layered_image_available(
                &workspace_path,
                agent_name,
                config.upgrade,
                &config.force_layers,
                config.isolated,
                config.verbose,
            )
            .await?
        } else if config.base_image == image::DEFAULT_IMAGE_NAME {
            // Default image should use layered system
            // If use_layered_images is false, it's likely a configuration error
            return Err(JailError::Backend(
                    "Default image requires layered images to be enabled. Please set use_layered_images to true.".to_string()
                ));
        } else {
            // For custom images, check if they exist and pull if needed
            let image_exists = self.image_exists(&config.base_image).await?;
            if !image_exists {
                debug!("Image {} not found locally, pulling...", config.base_image);
                let mut pull_cmd = Command::new("podman");
                pull_cmd.arg("pull").arg(&config.base_image);

                run_command(&mut pull_cmd)
                    .await
                    .map_err(|e| JailError::Backend(format!("Failed to pull image: {e}")))?;
            } else {
                debug!("Using local image: {}", config.base_image);
            }
            config.base_image.clone()
        };

        // Create and start the container with the determined image
        let mut modified_config = config.clone();
        modified_config.base_image = actual_image.clone();
        let args = self.build_run_args(&modified_config);
        let mut cmd = Command::new("podman");
        cmd.args(&args);

        debug!("Creating container with args: {:?}", args);
        run_command(&mut cmd).await?;

        info!("Jail {} created successfully", config.name);
        Ok(())
    }

    async fn remove(&self, name: &str, remove_volume: bool) -> Result<()> {
        info!("Removing podman jail: {}", name);

        // Remove container (with force flag to stop if running)
        let mut cmd = Command::new("podman");
        cmd.arg("rm").arg("-f").arg(name);

        run_command(&mut cmd).await?;

        if remove_volume {
            // Remove associated home volume
            let home_volume = format!("{name}-home");
            let mut vol_cmd = Command::new("podman");
            vol_cmd.arg("volume").arg("rm").arg(&home_volume);

            // Ignore errors if volume doesn't exist
            let _ = run_command(&mut vol_cmd).await;

            info!("Jail {} removed (including volume {})", name, home_volume);
        } else {
            info!("Jail {} removed", name);
        }

        Ok(())
    }

    async fn exec(&self, name: &str, command: &[String], interactive: bool) -> Result<String> {
        debug!(
            "Executing command in jail {}: {:?} (interactive: {})",
            name, command, interactive
        );

        // Check if container exists and is stopped
        if self.exists(name).await? {
            // Check container state
            let mut state_cmd = Command::new("podman");
            state_cmd
                .arg("inspect")
                .arg(name)
                .arg("--format")
                .arg("{{.State.Status}}");

            if let Ok(state) = run_command(&mut state_cmd).await {
                let state = state.trim();
                if state == "exited" || state == "stopped" || state == "created" {
                    info!("Container {} is {}, starting it...", name, state);
                    let mut start_cmd = Command::new("podman");
                    start_cmd.arg("start").arg(name);
                    run_command(&mut start_cmd).await?;
                    info!("Container {} started successfully", name);
                }
            }
        }

        let mut cmd = Command::new("podman");
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
                JailError::Backend(format!("Failed to execute interactive command: {e}"))
            })?;

            if !status.success() {
                return Err(JailError::ExecutionFailed(format!(
                    "Interactive command failed with status: {status}"
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
        let mut cmd = Command::new("podman");
        cmd.arg("ps")
            .arg("-a")
            .arg("--filter")
            .arg(format!("name={name}"))
            .arg("--format")
            .arg("{{.Names}}");

        match run_command(&mut cmd).await {
            Ok(output) => Ok(output.trim() == name),
            Err(_) => Ok(false),
        }
    }

    async fn list_all(&self) -> Result<Vec<String>> {
        debug!("Listing all jail-ai containers");

        let mut cmd = Command::new("podman");
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

    async fn inspect(&self, name: &str) -> Result<JailConfig> {
        debug!("Inspecting jail: {}", name);

        if !self.exists(name).await? {
            return Err(JailError::NotFound(format!("Jail '{name}' not found")));
        }

        let mut cmd = Command::new("podman");
        cmd.arg("inspect").arg(name).arg("--format").arg("json");

        let output = run_command(&mut cmd).await?;
        let inspect_data: serde_json::Value = serde_json::from_str(&output)
            .map_err(|e| JailError::Backend(format!("Failed to parse inspect output: {e}")))?;

        // Extract the first element if it's an array
        let container = inspect_data
            .as_array()
            .and_then(|arr| arr.first())
            .ok_or_else(|| JailError::Backend("Empty inspect output".to_string()))?;

        // Extract configuration
        let image = container["Config"]["Image"]
            .as_str()
            .unwrap_or(image::DEFAULT_IMAGE_NAME)
            .to_string();

        // Extract mounts
        let mut bind_mounts = Vec::new();
        if let Some(mounts) = container["Mounts"].as_array() {
            for mount in mounts {
                if mount["Type"].as_str() == Some("bind") {
                    let source = mount["Source"].as_str().unwrap_or("").to_string();
                    let destination = mount["Destination"].as_str().unwrap_or("").to_string();
                    let readonly = mount["RW"].as_bool().map(|rw| !rw).unwrap_or(false);

                    if !source.is_empty() && !destination.is_empty() {
                        bind_mounts.push(crate::config::BindMount {
                            source: source.into(),
                            target: destination.into(),
                            readonly,
                        });
                    }
                }
            }
        }

        // Extract environment variables
        let mut environment = Vec::new();
        if let Some(env_array) = container["Config"]["Env"].as_array() {
            for env in env_array {
                if let Some(env_str) = env.as_str() {
                    if let Some(pos) = env_str.find('=') {
                        let key = env_str[..pos].to_string();
                        let value = env_str[pos + 1..].to_string();
                        // Skip system environment variables
                        if !key.starts_with("PATH") && !key.starts_with("HOME") && key != "HOSTNAME"
                        {
                            environment.push((key, value));
                        }
                    }
                }
            }
        }

        // Extract network settings
        let network_mode = container["HostConfig"]["NetworkMode"]
            .as_str()
            .unwrap_or("default");
        let network = crate::config::NetworkConfig {
            enabled: network_mode != "none",
            private: network_mode == "private" || network_mode == "slirp4netns",
        };

        // Extract resource limits
        let memory_mb = container["HostConfig"]["Memory"]
            .as_i64()
            .filter(|&m| m > 0)
            .map(|m| (m / 1024 / 1024) as u64);

        let cpu_quota = container["HostConfig"]["CpuQuota"]
            .as_i64()
            .filter(|&q| q > 0)
            .and_then(|quota| {
                container["HostConfig"]["CpuPeriod"]
                    .as_i64()
                    .map(|period| ((quota as f64 / period as f64) * 100.0) as u32)
            });

        Ok(JailConfig {
            name: name.to_string(),
            backend: crate::config::BackendType::Podman,
            base_image: image,
            bind_mounts,
            environment,
            network,
            limits: crate::config::ResourceLimits {
                memory_mb,
                cpu_quota,
            },
            upgrade: false,
            force_layers: Vec::new(),
            use_layered_images: true,
            isolated: false,
            verbose: false,
        })
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
            upgrade: false,
            force_layers: Vec::new(),
            use_layered_images: true,
            isolated: false,
            verbose: false,
        };

        let args = backend.build_run_args(&config);

        assert!(args.contains(&"run".to_string()));
        assert!(args.contains(&"--name".to_string()));
        assert!(args.contains(&"test-jail".to_string()));
        assert!(args.contains(&"-m".to_string()));
        assert!(args.contains(&"512m".to_string()));
        assert!(args.contains(&"-e".to_string()));
        assert!(args.contains(&"TEST=value".to_string()));
        // Verify persistent home volume is included
        assert!(args.contains(&"test-jail-home:/home/agent".to_string()));
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
