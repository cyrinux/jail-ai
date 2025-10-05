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

async fn run(command: Commands) -> error::Result<()> {
    match command {
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

                let mut builder = JailBuilder::new(name)
                    .backend(backend_type)
                    .base_image(image)
                    .network(!no_network, true);

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
                print!("Remove jail '{}'? [y/N] ", name);
                use std::io::{self, BufRead};
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

        Commands::Exec { name, command } => {
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
            let output = jail.exec(&command).await?;
            print!("{}", output);
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
    }

    Ok(())
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
