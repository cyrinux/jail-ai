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

    /// Check if an image uses Nix by examining its name/tag
    /// Images with Nix will have "nix" in their layer tag or be the nix base image
    fn image_uses_nix(image: &str) -> bool {
        // Check if it's the Nix base image
        if image.contains("jail-ai-nix:") {
            return true;
        }
        
        // Check if it's an agent/project image with nix in the tag
        // Format: localhost/jail-ai-agent-claude:base-nix-rust or base-rust-nix
        if let Some(tag_part) = image.split(':').nth(1) {
            // Check if "nix" appears as a layer component
            // Valid: "base-nix", "base-nix-rust", "base-rust-nix"
            // Invalid: "phoenix" (contains "nix" but not as a separate layer)
            for component in tag_part.split('-') {
                if component == "nix" {
                    return true;
                }
            }
        }
        
        false
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

    /// Extract base name by stripping agent suffix
    /// Jail name format: jail__{project}__{hash}__{agent}
    /// Returns: jail__{project}__{hash}
    ///
    /// Simply strips the last segment after __ (the agent name).
    /// If there's no __, returns the name as-is (e.g., test names).
    fn extract_base_name(name: &str) -> String {
        if let Some(pos) = name.rfind("__") {
            // Strip the last segment (agent name)
            name[..pos].to_string()
        } else {
            // No __ found, return as-is (simple test names like "test")
            name.to_string()
        }
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

        // Extract base name (strip agent suffix if present)
        // jail__project__abc123__claude -> jail__project__abc123
        let base_name = Self::extract_base_name(&config.name);

        // Persistent volume for /home/agent to preserve data across upgrades
        // Shared across all agents working on the same project
        let home_volume = format!("{}__home", base_name);
        args.push("-v".to_string());
        args.push(format!("{home_volume}:/home/agent"));

        // Per-jail Nix store volume for containers using Nix
        // Shared across all agents working on the same project
        if Self::image_uses_nix(&config.base_image) {
            let nix_volume = format!("{}__nix", base_name);
            debug!("Detected Nix in image, mounting per-jail Nix store volume: {}", nix_volume);
            args.push("-v".to_string());
            args.push(format!("{nix_volume}:/nix"));
        }

        // Network settings
        // Supports: no network, host network, private network (slirp4netns), or default bridge
        if !config.network.enabled {
            // Complete network isolation
            args.push("--network=none".to_string());
        } else if config.network.host {
            // Host networking: container shares host's network namespace
            // Used for OAuth authentication to allow callbacks to localhost
            args.push("--network=host".to_string());
        } else if config.network.private {
            // Private networking with slirp4netns: secure, isolated, supports port forwarding
            args.push("--network=private".to_string());
        } else {
            // Shared networking (uses default bridge network)
            // Note: This mode is less isolated but allows container-to-container communication
        }

        // Port mappings (requires network to be enabled)
        // With private networking, port forwarding works correctly for OAuth callbacks
        if config.network.enabled {
            for port_mapping in &config.port_mappings {
                args.push("-p".to_string());
                args.push(format!(
                    "{}:{}/{}",
                    port_mapping.host_port, port_mapping.container_port, port_mapping.protocol
                ));
            }
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
                    "claude" | "copilot" | "cursor" | "gemini" | "jules" | "codex" => Some(suffix),
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
                config.skip_nix,
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

        // Pre-create directories if needed (for worktree support)
        if !config.pre_create_dirs.is_empty() {
            info!(
                "Pre-creating {} directories in container for worktree support",
                config.pre_create_dirs.len()
            );

            for dir in &config.pre_create_dirs {
                debug!("Creating directory: {}", dir.display());

                let mut mkdir_cmd = Command::new("podman");
                mkdir_cmd
                    .arg("exec")
                    .arg(&config.name)
                    .arg("mkdir")
                    .arg("-p")
                    .arg(dir.to_str().ok_or_else(|| {
                        JailError::Backend(format!(
                            "Invalid directory path: {}",
                            dir.display()
                        ))
                    })?);

                run_command(&mut mkdir_cmd).await.map_err(|e| {
                    JailError::Backend(format!(
                        "Failed to create directory {} in container: {}",
                        dir.display(),
                        e
                    ))
                })?;
            }

            info!("Pre-created directories successfully");
        }

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
            // Extract base name (strip agent suffix if present)
            let base_name = Self::extract_base_name(name);

            // Remove associated home volume
            let home_volume = format!("{base_name}__home");
            let mut vol_cmd = Command::new("podman");
            vol_cmd.arg("volume").arg("rm").arg(&home_volume);

            // Attempt removal but ignore errors (volume may be in use by other agents or not exist)
            match run_command(&mut vol_cmd).await {
                Ok(_) => debug!("Volume {} removed", home_volume),
                Err(e) => debug!(
                    "Could not remove volume {} (may be in use by other agents or not exist): {}",
                    home_volume, e
                ),
            }

            // Remove associated nix volume (if it exists)
            let nix_volume = format!("{base_name}__nix");
            let mut nix_vol_cmd = Command::new("podman");
            nix_vol_cmd.arg("volume").arg("rm").arg(&nix_volume);

            // Attempt removal but ignore errors (volume may be in use by other agents or not exist)
            match run_command(&mut nix_vol_cmd).await {
                Ok(_) => debug!("Volume {} removed", nix_volume),
                Err(e) => debug!(
                    "Could not remove volume {} (may be in use by other agents or not exist): {}",
                    nix_volume, e
                ),
            }

            info!(
                "Jail {} removed (attempted to remove volumes {}, {})",
                name, home_volume, nix_volume
            );
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

        // Filter containers that start with "jail__"
        let jails: Vec<String> = output
            .lines()
            .filter(|line| line.starts_with("jail__"))
            .map(|line| line.to_string())
            .collect();

        debug!("Found {} jail-ai containers", jails.len());
        Ok(jails)
    }

    async fn is_running(&self, name: &str) -> Result<bool> {
        let mut cmd = Command::new("podman");
        cmd.arg("ps")
            .arg("--filter")
            .arg(format!("name={name}"))
            .arg("--format")
            .arg("{{.Names}}");

        match run_command(&mut cmd).await {
            Ok(output) => Ok(output.trim() == name),
            Err(_) => Ok(false),
        }
    }

    async fn start(&self, name: &str) -> Result<()> {
        info!("Starting container: {}", name);

        let mut cmd = Command::new("podman");
        cmd.arg("start").arg(name);

        run_command(&mut cmd).await?;
        info!("Container {} started successfully", name);
        Ok(())
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
            // Check for both "slirp4netns" and "private" for backward compatibility
            // (older versions incorrectly used "private" which isn't a standard mode)
            private: network_mode == "slirp4netns" || network_mode == "private" || network_mode == "bridge",
            host: network_mode == "host",
        };

        // Extract port mappings
        let mut port_mappings = Vec::new();
        if let Some(port_bindings) = container["HostConfig"]["PortBindings"].as_object() {
            for (container_port_proto, bindings) in port_bindings {
                // container_port_proto format: "5432/tcp"
                let parts: Vec<&str> = container_port_proto.split('/').collect();
                if parts.len() == 2 {
                    if let Ok(container_port) = parts[0].parse::<u16>() {
                        let protocol = parts[1].to_string();
                        // bindings is an array of host port bindings
                        if let Some(bindings_array) = bindings.as_array() {
                            for binding in bindings_array {
                                if let Some(host_port_str) = binding["HostPort"].as_str() {
                                    if let Ok(host_port) = host_port_str.parse::<u16>() {
                                        port_mappings.push(crate::config::PortMapping {
                                            host_port,
                                            container_port,
                                            protocol: protocol.clone(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

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
            port_mappings,
            limits: crate::config::ResourceLimits {
                memory_mb,
                cpu_quota,
            },
            upgrade: false,
            force_layers: Vec::new(),
            use_layered_images: true,
            isolated: false,
            verbose: false,
            pre_create_dirs: Vec::new(), // Not persisted in container metadata
            skip_nix: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_run_args() {
        let backend = PodmanBackend::new();

        // Test with simple jail name (no __ suffix)
        let config = JailConfig {
            name: "test".to_string(),
            backend: crate::config::BackendType::Podman,
            base_image: "alpine:latest".to_string(),
            bind_mounts: vec![],
            environment: vec![("TEST".to_string(), "value".to_string())],
            network: crate::config::NetworkConfig {
                enabled: false,
                private: true,
                host: false,
            },
            port_mappings: vec![],
            limits: crate::config::ResourceLimits {
                memory_mb: Some(512),
                cpu_quota: Some(50),
            },
            upgrade: false,
            force_layers: Vec::new(),
            use_layered_images: true,
            isolated: false,
            verbose: false,
            pre_create_dirs: Vec::new(),
            skip_nix: false,
        };

        let args = backend.build_run_args(&config);

        assert!(args.contains(&"run".to_string()));
        assert!(args.contains(&"--name".to_string()));
        assert!(args.contains(&"test".to_string()));
        assert!(args.contains(&"-m".to_string()));
        assert!(args.contains(&"512m".to_string()));
        assert!(args.contains(&"-e".to_string()));
        assert!(args.contains(&"TEST=value".to_string()));
        // Verify persistent home volume is included
        // test -> test (no __ to strip, returns as-is)
        assert!(args.contains(&"test__home:/home/agent".to_string()));

        // Test with agent-specific jail name (8-char hash)
        let config_agent = JailConfig {
            name: "jail__project__abc12345__claude".to_string(),
            ..config.clone()
        };

        let args_agent = backend.build_run_args(&config_agent);

        // Verify volume uses base name without agent suffix
        // jail__project__abc12345__claude -> jail__project__abc12345
        assert!(args_agent.contains(&"jail__project__abc12345__home:/home/agent".to_string()));
        assert!(!args_agent.contains(&"jail__project__abc12345__claude__home:/home/agent".to_string()));
    }

    #[test]
    fn test_build_run_args_with_port_mappings() {
        let backend = PodmanBackend::new();
        let config = JailConfig {
            name: "test-jail".to_string(),
            backend: crate::config::BackendType::Podman,
            base_image: "alpine:latest".to_string(),
            bind_mounts: vec![],
            environment: vec![],
            network: crate::config::NetworkConfig {
                enabled: true,
                private: true,
                host: false,
            },
            port_mappings: vec![
                crate::config::PortMapping {
                    host_port: 8080,
                    container_port: 80,
                    protocol: "tcp".to_string(),
                },
                crate::config::PortMapping {
                    host_port: 5432,
                    container_port: 5432,
                    protocol: "tcp".to_string(),
                },
            ],
            limits: crate::config::ResourceLimits {
                memory_mb: None,
                cpu_quota: None,
            },
            upgrade: false,
            force_layers: Vec::new(),
            use_layered_images: true,
            isolated: false,
            verbose: false,
            skip_nix: false,
            pre_create_dirs: Vec::new(),
        };

        let args = backend.build_run_args(&config);

        // Verify port mappings are included
        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"8080:80/tcp".to_string()));
        assert!(args.contains(&"5432:5432/tcp".to_string()));
    }

    #[test]
    fn test_build_run_args_port_mappings_require_network() {
        let backend = PodmanBackend::new();
        let config = JailConfig {
            name: "test-jail".to_string(),
            backend: crate::config::BackendType::Podman,
            base_image: "alpine:latest".to_string(),
            bind_mounts: vec![],
            environment: vec![],
            network: crate::config::NetworkConfig {
                enabled: false,
                private: true,
                host: false,
            },
            port_mappings: vec![crate::config::PortMapping {
                host_port: 8080,
                container_port: 80,
                protocol: "tcp".to_string(),
            }],
            limits: crate::config::ResourceLimits {
                memory_mb: None,
                cpu_quota: None,
            },
            upgrade: false,
            force_layers: Vec::new(),
            use_layered_images: true,
            isolated: false,
            verbose: false,
            skip_nix: false,
            pre_create_dirs: Vec::new(),
        };

        let args = backend.build_run_args(&config);

        // Port mappings should NOT be included when network is disabled
        let port_args_count = args.iter().filter(|&arg| arg == "-p").count();
        assert_eq!(port_args_count, 0);
    }

    #[test]
    fn test_list_all_filters_jail_prefix() {
        // This is a unit test that verifies the filtering logic would work
        let names = vec![
            "jail__project__def67890__claude",
            "other-container",
            "jail__another__xyz12345__copilot",
            "my-container",
        ];

        let filtered: Vec<String> = names
            .into_iter()
            .filter(|name| name.starts_with("jail__"))
            .map(|s| s.to_string())
            .collect();

        assert_eq!(filtered.len(), 2);
        assert!(filtered.contains(&"jail__project__def67890__claude".to_string()));
        assert!(filtered.contains(&"jail__another__xyz12345__copilot".to_string()));
    }

    #[test]
    fn test_image_uses_nix() {
        // Test Nix base image
        assert!(PodmanBackend::image_uses_nix("localhost/jail-ai-nix:latest"));

        // Test agent images with Nix in layer tag
        assert!(PodmanBackend::image_uses_nix("localhost/jail-ai-agent-claude:base-nix"));
        assert!(PodmanBackend::image_uses_nix("localhost/jail-ai-agent-claude:base-nix-rust"));
        assert!(PodmanBackend::image_uses_nix("localhost/jail-ai-agent-claude:base-rust-nix"));
        assert!(PodmanBackend::image_uses_nix("localhost/jail-ai-agent-jules:base-rust-nix-nodejs"));

        // Test images without Nix
        assert!(!PodmanBackend::image_uses_nix("localhost/jail-ai-agent-claude:base"));
        assert!(!PodmanBackend::image_uses_nix("localhost/jail-ai-agent-claude:base-rust"));
        assert!(!PodmanBackend::image_uses_nix("localhost/jail-ai-agent-claude:base-rust-nodejs"));
        assert!(!PodmanBackend::image_uses_nix("alpine:latest"));

        // Test that "nix" substring in other words doesn't trigger false positive
        assert!(!PodmanBackend::image_uses_nix("localhost/phoenix-app:latest"));
        assert!(!PodmanBackend::image_uses_nix("localhost/jail-ai-agent-claude:base-unix"));
    }

    #[test]
    fn test_extract_base_name() {
        // Test extracting base name from agent-specific jail names
        // Hash is always 8 hex characters (as per generate_jail_name)
        // Works for any agent name (future-proof)
        assert_eq!(
            PodmanBackend::extract_base_name("jail__project__abc12345__claude"),
            "jail__project__abc12345"
        );
        assert_eq!(
            PodmanBackend::extract_base_name("jail__project__abc12345__copilot"),
            "jail__project__abc12345"
        );
        assert_eq!(
            PodmanBackend::extract_base_name("jail__project__def67890__cursor"),
            "jail__project__def67890"
        );
        assert_eq!(
            PodmanBackend::extract_base_name("jail__project__12345678__gemini"),
            "jail__project__12345678"
        );
        assert_eq!(
            PodmanBackend::extract_base_name("jail__myproject__abcdef12__jules"),
            "jail__myproject__abcdef12"
        );
        assert_eq!(
            PodmanBackend::extract_base_name("jail__test__fedcba98__codex"),
            "jail__test__fedcba98"
        );

        // Test with any future agent name (strips last segment after __)
        assert_eq!(
            PodmanBackend::extract_base_name("jail__project__abc12345__newagent"),
            "jail__project__abc12345"
        );

        // Test simple name without double underscores (test names)
        assert_eq!(
            PodmanBackend::extract_base_name("test"),
            "test"
        );
    }

    #[test]
    fn test_build_run_args_with_nix_volume() {
        let backend = PodmanBackend::new();

        // Test with Nix-enabled image and agent-specific jail name (8-char hash)
        let config_with_nix = JailConfig {
            name: "jail__project__abc12345__claude".to_string(),
            backend: crate::config::BackendType::Podman,
            base_image: "localhost/jail-ai-agent-claude:base-nix-rust".to_string(),
            bind_mounts: vec![],
            environment: vec![],
            network: crate::config::NetworkConfig {
                enabled: true,
                private: true,
                host: false,
            },
            port_mappings: vec![],
            limits: crate::config::ResourceLimits {
                memory_mb: None,
                cpu_quota: None,
            },
            upgrade: false,
            force_layers: Vec::new(),
            use_layered_images: true,
            isolated: false,
            verbose: false,
            skip_nix: false,
            pre_create_dirs: Vec::new(),
        };

        let args = backend.build_run_args(&config_with_nix);

        // Verify both home and nix volumes use base name (without agent suffix)
        // jail__project__abc12345__claude -> jail__project__abc12345
        assert!(args.contains(&"jail__project__abc12345__home:/home/agent".to_string()));
        assert!(args.contains(&"jail__project__abc12345__nix:/nix".to_string()));

        // Verify agent-specific volume names are NOT used
        assert!(!args.contains(&"jail__project__abc12345__claude__home:/home/agent".to_string()));
        assert!(!args.contains(&"jail__project__abc12345__claude__nix:/nix".to_string()));

        // Test with different agent but same project (should share volumes)
        let config_with_copilot = JailConfig {
            name: "jail__project__abc12345__copilot".to_string(),
            ..config_with_nix.clone()
        };

        let args_copilot = backend.build_run_args(&config_with_copilot);

        // Verify both agents use the same base volume names
        assert!(args_copilot.contains(&"jail__project__abc12345__home:/home/agent".to_string()));
        assert!(args_copilot.contains(&"jail__project__abc12345__nix:/nix".to_string()));

        // Test without Nix image
        let config_without_nix = JailConfig {
            base_image: "alpine:latest".to_string(),
            ..config_with_nix
        };

        let args = backend.build_run_args(&config_without_nix);

        // Verify only home volume is mounted (no nix volume)
        assert!(args.contains(&"jail__project__abc12345__home:/home/agent".to_string()));
        assert!(!args.contains(&"jail__project__abc12345__nix:/nix".to_string()));

        // Verify the old shared volume name is NOT used
        assert!(!args.iter().any(|arg| arg.contains("jail-ai-nix-store")));
    }
}
