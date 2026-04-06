use super::{run_command, JailBackend};
use crate::config::JailConfig;
use crate::error::{JailError, Result};
use crate::image;
use async_trait::async_trait;
use tokio::process::Command;
use tracing::{debug, info, warn};

pub struct ContainerAppBackend;

impl ContainerAppBackend {
    pub fn new() -> Self {
        Self
    }

    async fn image_exists(&self, image: &str) -> Result<bool> {
        let mut cmd = Command::new("container");
        cmd.arg("images").arg("--quiet").arg(image);

        match cmd.output().await {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                Ok(output.status.success() && !stdout.trim().is_empty())
            }
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

        if !config.network.enabled {
            args.push("--network=none".to_string());
        }

        for mount in &config.bind_mounts {
            let bind_arg = if mount.readonly {
                format!("{}:{}:ro", mount.source.display(), mount.target.display())
            } else {
                format!("{}:{}", mount.source.display(), mount.target.display())
            };
            args.push("-v".to_string());
            args.push(bind_arg);
        }

        for (key, value) in &config.environment {
            args.push("-e".to_string());
            args.push(format!("{key}={value}"));
        }

        if let Some(memory_mb) = config.limits.memory_mb {
            args.push("-m".to_string());
            args.push(format!("{memory_mb}m"));
        }

        args.push(config.base_image.clone());

        args.push("sleep".to_string());
        args.push("infinity".to_string());

        args
    }
}

impl Default for ContainerAppBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl JailBackend for ContainerAppBackend {
    async fn create(&self, config: &JailConfig) -> Result<()> {
        info!("Creating container jail (apple/container): {}", config.name);

        if config.upgrade && self.exists(&config.name).await? {
            info!(
                "Upgrade enabled: removing existing container '{}'",
                config.name
            );
            let mut rm_cmd = Command::new("container");
            rm_cmd.arg("rm").arg("-f").arg(&config.name);
            if let Err(e) = run_command(&mut rm_cmd).await {
                debug!("Failed to remove existing container (may not exist): {}", e);
            }
        } else if !config.upgrade && self.exists(&config.name).await? {
            return Err(JailError::AlreadyExists(config.name.clone()));
        }

        let actual_image =
            if config.base_image == image::DEFAULT_IMAGE_NAME && config.use_layered_images {
                info!("Using layered image system with auto-detection");

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

                let agent_name = config.name.rsplit("__").next().and_then(|suffix| {
                    crate::agents::Agent::from_str(suffix).map(|a| a.normalized_name())
                });

                debug!(
                    "Workspace path: {:?}, Agent: {:?}",
                    workspace_path, agent_name
                );

                crate::image_layers::ensure_layered_image_available(
                    &workspace_path,
                    agent_name,
                    config.upgrade,
                    &config.force_layers,
                    config.isolated,
                    config.verbose,
                    config.no_nix,
                )
                .await?
            } else if config.base_image == image::DEFAULT_IMAGE_NAME {
                return Err(JailError::Backend(
                    "Default image requires layered images to be enabled.".to_string(),
                ));
            } else {
                let image_exists = self.image_exists(&config.base_image).await?;
                if !image_exists {
                    debug!("Image {} not found locally, pulling...", config.base_image);
                    let mut pull_cmd = Command::new("container");
                    pull_cmd.arg("pull").arg(&config.base_image);
                    run_command(&mut pull_cmd)
                        .await
                        .map_err(|e| JailError::Backend(format!("Failed to pull image: {e}")))?;
                }
                config.base_image.clone()
            };

        let mut modified_config = config.clone();
        modified_config.base_image = actual_image;
        let args = self.build_run_args(&modified_config);
        let mut cmd = Command::new("container");
        cmd.args(&args);

        debug!("Creating container with args: {:?}", args);
        run_command(&mut cmd).await?;

        if config.block_host {
            warn!(
                "eBPF host blocking is not supported on macOS (apple/container backend), skipping for container '{}'",
                config.name
            );
        }

        if !config.pre_create_dirs.is_empty() {
            for dir in &config.pre_create_dirs {
                debug!("Creating directory: {}", dir.display());
                let mut mkdir_cmd = Command::new("container");
                mkdir_cmd
                    .arg("exec")
                    .arg(&config.name)
                    .arg("mkdir")
                    .arg("-p")
                    .arg(dir.to_str().ok_or_else(|| {
                        JailError::Backend(format!("Invalid directory path: {}", dir.display()))
                    })?);
                run_command(&mut mkdir_cmd).await.map_err(|e| {
                    JailError::Backend(format!(
                        "Failed to create directory {} in container: {}",
                        dir.display(),
                        e
                    ))
                })?;
            }
        }

        info!("Jail {} created successfully", config.name);
        Ok(())
    }

    async fn remove(&self, name: &str, remove_volume: bool) -> Result<()> {
        info!("Removing container jail: {}", name);

        let mut cmd = Command::new("container");
        cmd.arg("rm").arg("-f").arg(name);
        run_command(&mut cmd).await?;

        if remove_volume {
            info!(
                "Note: volume removal is not supported by apple/container backend for '{}'",
                name
            );
        }

        info!("Jail {} removed", name);
        Ok(())
    }

    async fn exec(&self, name: &str, command: &[String], interactive: bool) -> Result<String> {
        debug!(
            "Executing command in jail {}: {:?} (interactive: {})",
            name, command, interactive
        );

        if self.exists(name).await? {
            let mut state_cmd = Command::new("container");
            state_cmd
                .arg("inspect")
                .arg(name)
                .arg("--format")
                .arg("{{.State.Status}}");

            if let Ok(state) = run_command(&mut state_cmd).await {
                let state = state.trim();
                if state == "exited" || state == "stopped" || state == "created" {
                    info!("Container {} is {}, starting it...", name, state);
                    let mut start_cmd = Command::new("container");
                    start_cmd.arg("start").arg(name);
                    run_command(&mut start_cmd).await?;
                    info!("Container {} started successfully", name);
                }
            }
        }

        let mut cmd = Command::new("container");
        cmd.arg("exec");

        if interactive {
            cmd.arg("-it");
        }

        cmd.arg(name);

        for arg in command {
            cmd.arg(arg);
        }

        if interactive {
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

            Ok(String::new())
        } else {
            let output = run_command(&mut cmd).await?;
            debug!("Command output: {}", output);
            Ok(output)
        }
    }

    async fn exists(&self, name: &str) -> Result<bool> {
        let mut cmd = Command::new("container");
        cmd.arg("ps")
            .arg("-a")
            .arg("--filter")
            .arg(format!("name={name}"))
            .arg("--format")
            .arg("{{.Names}}");

        match run_command(&mut cmd).await {
            Ok(output) => Ok(output.lines().any(|line| line.trim() == name)),
            Err(_) => Ok(false),
        }
    }

    async fn list_all(&self) -> Result<Vec<String>> {
        debug!("Listing all jail-ai containers");

        let mut cmd = Command::new("container");
        cmd.arg("ps").arg("-a").arg("--format").arg("{{.Names}}");

        let output = run_command(&mut cmd).await?;

        let jails: Vec<String> = output
            .lines()
            .filter(|line| line.starts_with("jail__"))
            .map(|line| line.to_string())
            .collect();

        debug!("Found {} jail-ai containers", jails.len());
        Ok(jails)
    }

    async fn is_running(&self, name: &str) -> Result<bool> {
        let mut cmd = Command::new("container");
        cmd.arg("ps")
            .arg("--filter")
            .arg(format!("name={name}"))
            .arg("--format")
            .arg("{{.Names}}");

        match run_command(&mut cmd).await {
            Ok(output) => Ok(output.lines().any(|line| line.trim() == name)),
            Err(_) => Ok(false),
        }
    }

    async fn start(&self, name: &str) -> Result<()> {
        info!("Starting container: {}", name);
        let mut cmd = Command::new("container");
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

        let mut cmd = Command::new("container");
        cmd.arg("inspect").arg(name).arg("--format").arg("json");

        let output = run_command(&mut cmd).await?;
        let inspect_data: serde_json::Value = serde_json::from_str(&output)
            .map_err(|e| JailError::Backend(format!("Failed to parse inspect output: {e}")))?;

        let container = if let Some(arr) = inspect_data.as_array() {
            arr.first()
                .ok_or_else(|| JailError::Backend("Empty inspect output".to_string()))?
                .clone()
        } else {
            inspect_data
        };

        let image_val = container["Config"]["Image"]
            .as_str()
            .or_else(|| container["Image"].as_str())
            .unwrap_or(image::DEFAULT_IMAGE_NAME)
            .to_string();

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

        let mut environment = Vec::new();
        if let Some(env_array) = container["Config"]["Env"].as_array() {
            for env in env_array {
                if let Some(env_str) = env.as_str() {
                    if let Some(pos) = env_str.find('=') {
                        let key = env_str[..pos].to_string();
                        let value = env_str[pos + 1..].to_string();
                        if !key.starts_with("PATH") && !key.starts_with("HOME") && key != "HOSTNAME"
                        {
                            environment.push((key, value));
                        }
                    }
                }
            }
        }

        let network_mode = container["HostConfig"]["NetworkMode"]
            .as_str()
            .unwrap_or("default");
        let network = crate::config::NetworkConfig {
            enabled: network_mode != "none",
            private: false,
            host: network_mode == "host",
        };

        let memory_mb = container["HostConfig"]["Memory"]
            .as_i64()
            .filter(|&m| m > 0)
            .map(|m| (m / 1024 / 1024) as u64);

        Ok(JailConfig {
            name: name.to_string(),
            backend: crate::config::BackendType::ContainerApp,
            base_image: image_val,
            bind_mounts,
            environment,
            network,
            port_mappings: Vec::new(),
            limits: crate::config::ResourceLimits {
                memory_mb,
                cpu_quota: None,
            },
            upgrade: false,
            force_layers: Vec::new(),
            use_layered_images: true,
            isolated: false,
            verbose: false,
            pre_create_dirs: Vec::new(),
            no_nix: false,
            block_host: false,
            podman_socket: false,
        })
    }
}
