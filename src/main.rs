mod backend;
mod cli;
mod config;
mod error;
mod jail;

use clap::Parser;
use cli::{Cli, Commands};
use config::JailConfig;
use jail::JailBuilder;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = if cli.verbose {
        "jail_ai=debug,info"
    } else {
        "jail_ai=info"
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| filter.into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    if let Err(e) = run(cli.command).await {
        error!("Error: {}", e);
        std::process::exit(1);
    }
}

async fn run(command: Option<Commands>) -> error::Result<()> {
    match command {
        None => {
            // Default behavior: auto-init and exec based on current directory
            let cwd = std::env::current_dir()?;
            let dir_name = cwd
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("default");
            let jail_name = cli::Commands::sanitize_jail_name(dir_name);

            info!("No command specified, using default behavior for jail '{}'", jail_name);

            // Check if jail exists
            let temp_config = JailConfig {
                name: jail_name.clone(),
                ..Default::default()
            };
            let temp_jail = jail::JailManager::new(temp_config);
            let exists = temp_jail.exists().await?;

            if !exists {
                info!("Jail '{}' does not exist, creating it...", jail_name);

                // Create jail with default settings
                let mut builder = JailBuilder::new(jail_name.clone())
                    .backend(config::BackendType::Podman)
                    .base_image("localhost/jail-ai-env:latest".to_string());

                // Set timezone from host
                if let Some(tz) = get_host_timezone() {
                    builder = builder.env("TZ", tz);
                }

                // Auto-mount workspace
                builder = builder.bind_mount(cwd.clone(), "/workspace", false);

                // Auto-mount AI agent configs
                let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
                let home_path = std::path::PathBuf::from(&home);

                let claude_config = home_path.join(".claude");
                if claude_config.exists() {
                    builder = builder.bind_mount(claude_config, "/home/agent/.claude", false);
                }

                let claude_json = home_path.join(".claude.json");
                if claude_json.exists() {
                    builder = builder.bind_mount(claude_json, "/home/agent/.claude.json", false);
                }

                let config_dir = home_path.join(".config");
                if config_dir.exists() {
                    builder = builder.bind_mount(config_dir, "/home/agent/.config", false);
                }

                let cursor_config = home_path.join(".cursor");
                if cursor_config.exists() {
                    builder = builder.bind_mount(cursor_config, "/home/agent/.cursor", false);
                }

                let jail = builder.build();
                jail.create().await?;
                info!("Jail '{}' created successfully", jail_name);
            }

            // Exec into jail with interactive shell
            info!("Executing interactive shell in jail '{}'...", jail_name);
            let jail = JailBuilder::new(jail_name.clone())
                .backend(config::BackendType::Podman)
                .build();

            jail.exec(&["/usr/bin/zsh".to_string()], true).await?;
        }
        Some(command) => match command {
        Commands::Create {
            name,
            backend,
            image,
            mount,
            env,
            no_network,
            memory,
            cpu,
            config,
            no_workspace,
            workspace_path,
            no_agent_configs,
        } => {
            let jail = if let Some(config_path) = config {
                // Load from config file
                let config_str = tokio::fs::read_to_string(&config_path).await?;
                let config: JailConfig = serde_json::from_str(&config_str)?;
                jail::JailManager::new(config)
            } else {
                // Build from CLI args
                let backend_type = Commands::parse_backend(&backend)
                    .map_err(error::JailError::Config)?;

                // Auto-generate name from current directory if not provided
                let jail_name = if let Some(name) = name {
                    // Sanitize user-provided name too
                    cli::Commands::sanitize_jail_name(&name)
                } else {
                    let cwd = std::env::current_dir()?;
                    let dir_name = cwd
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("default");
                    let generated_name = cli::Commands::sanitize_jail_name(dir_name);
                    info!("Auto-generated jail name from current directory: {}", generated_name);
                    generated_name
                };

                let mut builder = JailBuilder::new(jail_name)
                    .backend(backend_type)
                    .base_image(image)
                    .network(!no_network, true);

                // Set timezone from host
                if let Some(tz) = get_host_timezone() {
                    builder = builder.env("TZ", tz);
                }

                // Auto-mount current working directory to /workspace
                if !no_workspace {
                    let cwd = std::env::current_dir()?;
                    info!(
                        "Auto-mounting current directory {} to {}",
                        cwd.display(),
                        workspace_path
                    );
                    builder = builder.bind_mount(cwd, workspace_path, false);
                }

                // Auto-mount AI agent config directories
                if !no_agent_configs {
                    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
                    let home_path = std::path::PathBuf::from(&home);

                    // Mount ~/.claude for Claude Code
                    let claude_config = home_path.join(".claude");
                    if claude_config.exists() {
                        info!("Auto-mounting {} to /home/agent/.claude", claude_config.display());
                        builder = builder.bind_mount(claude_config, "/home/agent/.claude", false);
                    }

                    // Mount ~/.claude.json for Claude Code configuration
                    let claude_json = home_path.join(".claude.json");
                    if claude_json.exists() {
                        info!("Auto-mounting {} to /home/agent/.claude.json", claude_json.display());
                        builder = builder.bind_mount(claude_json, "/home/agent/.claude.json", false);
                    }

                    // Mount ~/.config for GitHub Copilot CLI and other tools
                    let config_dir = home_path.join(".config");
                    if config_dir.exists() {
                        info!("Auto-mounting {} to /home/agent/.config", config_dir.display());
                        builder = builder.bind_mount(config_dir, "/home/agent/.config", false);
                    }

                    // Mount ~/.cursor for Cursor Agent
                    let cursor_config = home_path.join(".cursor");
                    if cursor_config.exists() {
                        info!("Auto-mounting {} to /home/agent/.cursor", cursor_config.display());
                        builder = builder.bind_mount(cursor_config, "/home/agent/.cursor", false);
                    }
                }

                // Parse mounts
                for mount_str in mount {
                    let mount = Commands::parse_mount(&mount_str)
                        .map_err(error::JailError::Config)?;
                    builder = builder.bind_mount(mount.source, mount.target, mount.readonly);
                }

                // Parse environment variables
                for env_str in env {
                    let (key, value) = Commands::parse_env(&env_str)
                        .map_err(error::JailError::Config)?;
                    builder = builder.env(key, value);
                }

                // Set resource limits
                if let Some(mem) = memory {
                    builder = builder.memory_limit(mem);
                }
                if let Some(cpu_quota) = cpu {
                    builder = builder.cpu_quota(cpu_quota);
                }

                builder.build()
            };

            jail.create().await?;
            info!("Jail created: {}", jail.config().name);
        }

        Commands::Start { name } => {
            let config = JailConfig {
                name: name.clone(),
                ..Default::default()
            };
            let jail = jail::JailManager::new(config);
            jail.start().await?;
            info!("Jail started: {}", name);
        }

        Commands::Stop { name } => {
            let config = JailConfig {
                name: name.clone(),
                ..Default::default()
            };
            let jail = jail::JailManager::new(config);
            jail.stop().await?;
            info!("Jail stopped: {}", name);
        }

        Commands::Remove { name, force } => {
            if !force {
                use std::io::{self, BufRead, Write};
                print!("Remove jail '{}'? [y/N] ", name);
                io::stdout().flush()?;
                let stdin = io::stdin();
                let mut line = String::new();
                stdin.lock().read_line(&mut line)?;
                if !line.trim().eq_ignore_ascii_case("y") {
                    info!("Aborted");
                    return Ok(());
                }
            }

            let config = JailConfig {
                name: name.clone(),
                ..Default::default()
            };
            let jail = jail::JailManager::new(config);
            jail.remove().await?;
            info!("Jail removed: {}", name);
        }

        Commands::Exec { name, command, interactive } => {
            if command.is_empty() {
                return Err(error::JailError::Config(
                    "No command specified".to_string(),
                ));
            }

            let config = JailConfig {
                name: name.clone(),
                ..Default::default()
            };
            let jail = jail::JailManager::new(config);
            let output = jail.exec(&command, interactive).await?;

            // Only print output if not interactive (interactive mode outputs directly)
            if !interactive && !output.is_empty() {
                print!("{}", output);
            }
        }

        Commands::Status { name } => {
            let config = JailConfig {
                name: name.clone(),
                ..Default::default()
            };
            let jail = jail::JailManager::new(config);
            let exists = jail.exists().await?;
            if exists {
                info!("Jail '{}' exists", name);
            } else {
                info!("Jail '{}' does not exist", name);
            }
        }

        Commands::Save { name, output } => {
            let config = JailConfig {
                name,
                ..Default::default()
            };
            let json = serde_json::to_string_pretty(&config)?;
            tokio::fs::write(&output, json).await?;
            info!("Configuration saved to: {}", output.display());
        }

        Commands::Claude { args } => {
            let cwd = std::env::current_dir()?;
            let jail_name = cli::Commands::generate_jail_name(&cwd);

            info!("Using jail: {} for directory: {}", jail_name, cwd.display());

            // Create jail if it doesn't exist
            let config = JailConfig {
                name: jail_name.clone(),
                ..Default::default()
            };
            let jail = jail::JailManager::new(config.clone());

            if !jail.exists().await? {
                info!("Creating new jail: {}", jail_name);
                let jail = create_default_jail(&jail_name, &cwd).await?;
                jail.create().await?;
            }

            // Execute Claude Code
            let mut command = vec!["claude".to_string()];
            command.extend(args);

            let output = jail.exec(&command, true).await?;
            if !output.is_empty() {
                print!("{}", output);
            }
        }

        Commands::Copilot { args } => {
            let cwd = std::env::current_dir()?;
            let jail_name = cli::Commands::generate_jail_name(&cwd);

            info!("Using jail: {} for directory: {}", jail_name, cwd.display());

            // Create jail if it doesn't exist
            let config = JailConfig {
                name: jail_name.clone(),
                ..Default::default()
            };
            let jail = jail::JailManager::new(config.clone());

            if !jail.exists().await? {
                info!("Creating new jail: {}", jail_name);
                let jail = create_default_jail(&jail_name, &cwd).await?;
                jail.create().await?;
            }

            // Execute GitHub Copilot CLI
            let mut command = vec!["copilot".to_string()];
            command.extend(args);

            let output = jail.exec(&command, true).await?;
            if !output.is_empty() {
                print!("{}", output);
            }
        }

        Commands::Cursor { args } => {
            let cwd = std::env::current_dir()?;
            let jail_name = cli::Commands::generate_jail_name(&cwd);

            info!("Using jail: {} for directory: {}", jail_name, cwd.display());

            // Create jail if it doesn't exist
            let config = JailConfig {
                name: jail_name.clone(),
                ..Default::default()
            };
            let jail = jail::JailManager::new(config.clone());

            if !jail.exists().await? {
                info!("Creating new jail: {}", jail_name);
                let jail = create_default_jail(&jail_name, &cwd).await?;
                jail.create().await?;
            }

            // Execute Cursor Agent
            let mut command = vec!["cursor-agent".to_string()];
            command.extend(args);

            let output = jail.exec(&command, true).await?;
            if !output.is_empty() {
                print!("{}", output);
            }
        }

        Commands::Join { shell } => {
            let cwd = std::env::current_dir()?;
            let jail_name = cli::Commands::generate_jail_name(&cwd);

            info!("Joining jail: {} for directory: {}", jail_name, cwd.display());

            // Check if jail exists
            let config = JailConfig {
                name: jail_name.clone(),
                ..Default::default()
            };
            let jail = jail::JailManager::new(config);

            if !jail.exists().await? {
                return Err(error::JailError::Config(format!(
                    "Jail '{}' does not exist. Create it first with: jail-ai claude",
                    jail_name
                )));
            }

            // Execute interactive shell
            let command = vec![shell];
            let output = jail.exec(&command, true).await?;
            if !output.is_empty() {
                print!("{}", output);
            }
        }
        }
    }

    Ok(())
}

/// Get the host's timezone
fn get_host_timezone() -> Option<String> {
    // Try TZ environment variable first
    if let Ok(tz) = std::env::var("TZ")
        && !tz.is_empty() {
            info!("Using timezone from TZ env var: {}", tz);
            return Some(tz);
        }

    // Try reading /etc/timezone
    if let Ok(tz) = std::fs::read_to_string("/etc/timezone") {
        let tz = tz.trim().to_string();
        if !tz.is_empty() {
            info!("Using timezone from /etc/timezone: {}", tz);
            return Some(tz);
        }
    }

    // Try reading /etc/localtime symlink
    if let Ok(link) = std::fs::read_link("/etc/localtime")
        && let Some(tz) = link.to_str() {
            // Extract timezone from path like /usr/share/zoneinfo/Europe/Paris
            if let Some(tz_name) = tz.strip_prefix("/usr/share/zoneinfo/") {
                info!("Using timezone from /etc/localtime: {}", tz_name);
                return Some(tz_name.to_string());
            }
        }

    info!("Could not determine host timezone, container will use UTC");
    None
}

/// Helper function to create a jail with default configuration
async fn create_default_jail(name: &str, workspace: &std::path::Path) -> error::Result<jail::JailManager> {
    let backend_type = config::BackendType::Podman;

    let mut builder = JailBuilder::new(name)
        .backend(backend_type)
        .base_image("localhost/jail-ai-env:latest")
        .network(true, true);

    // Set timezone from host
    if let Some(tz) = get_host_timezone() {
        builder = builder.env("TZ", tz);
    }

    // Auto-mount workspace
    info!("Auto-mounting {} to /workspace", workspace.display());
    builder = builder.bind_mount(workspace, "/workspace", false);

    // Auto-mount AI agent configs
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let home_path = std::path::PathBuf::from(&home);

    // Mount ~/.claude for Claude Code
    let claude_config = home_path.join(".claude");
    if claude_config.exists() {
        info!("Auto-mounting {} to /home/agent/.claude", claude_config.display());
        builder = builder.bind_mount(claude_config, "/home/agent/.claude", false);
    }

    // Mount ~/.claude.json for Claude Code configuration
    let claude_json = home_path.join(".claude.json");
    if claude_json.exists() {
        info!("Auto-mounting {} to /home/agent/.claude.json", claude_json.display());
        builder = builder.bind_mount(claude_json, "/home/agent/.claude.json", false);
    }

    // Mount ~/.config for GitHub Copilot CLI and other tools
    let config_dir = home_path.join(".config");
    if config_dir.exists() {
        info!("Auto-mounting {} to /home/agent/.config", config_dir.display());
        builder = builder.bind_mount(config_dir, "/home/agent/.config", false);
    }

    // Mount ~/.cursor for Cursor Agent
    let cursor_config = home_path.join(".cursor");
    if cursor_config.exists() {
        info!("Auto-mounting {} to /home/agent/.cursor", cursor_config.display());
        builder = builder.bind_mount(cursor_config, "/home/agent/.cursor", false);
    }

    Ok(builder.build())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_jail_config_serialization() {
        let config = JailConfig {
            name: "test".to_string(),
            backend: config::BackendType::Podman,
            ..Default::default()
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: JailConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.name, deserialized.name);
        assert_eq!(config.backend, deserialized.backend);
    }
}
