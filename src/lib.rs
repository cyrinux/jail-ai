pub mod agent_commands;
pub mod agents;
pub mod backend;
pub mod cli;
pub mod config;
pub mod ebpf;
pub mod error;
pub mod git_gpg;
pub mod image;
pub mod image_layers;
pub mod image_parallel;
pub mod jail;
pub mod jail_detection;
pub mod jail_setup;
pub mod project_detection;
pub mod state;
pub mod tui;
pub mod upgrade;
pub mod worktree;

pub use cli::{Cli, Commands};
pub use config::JailConfig;
pub use error::{JailError, Result};
pub use project_detection::ProjectType;
pub use tracing_subscriber::layer::SubscriberExt;
pub use tracing_subscriber::util::SubscriberInitExt;
pub use clap::Parser;
pub use upgrade::{upgrade_single_jail, upgrade_all_jails, resolve_jail_name};

pub async fn run_cli() -> Result<()> {
    let cli = Cli::parse();

    let filter = if cli.verbose {
        "jail_ai=debug"
    } else {
        "jail_ai=warn"
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| filter.into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let command = cli.command;
    let verbose = cli.verbose;

    run(command, verbose).await
}

pub fn validate_mount_source(source: &std::path::Path) -> Result<()> {
    use tracing::debug;

    if !source.exists() {
        use std::io::{self, BufRead, Write};
        println!("⚠️  Mount source path does not exist: {}", source.display());
        print!("Create this path? [y/N] ");
        io::stdout().flush()?;
        let stdin = io::stdin();
        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;

        if line.trim().eq_ignore_ascii_case("y") {
            debug!("Creating mount source directory: {}", source.display());
            std::fs::create_dir_all(source).map_err(|e| {
                JailError::Config(format!(
                    "Failed to create mount source directory '{}': {}",
                    source.display(),
                    e
                ))
            })?;
            println!("✓ Created directory: {}", source.display());
        } else {
            return Err(JailError::Config(format!(
                "Mount source path does not exist: {}",
                source.display()
            )));
        }
    }

    let source = source.canonicalize().map_err(JailError::Io)?;

    if source == std::path::Path::new("/") {
        return Err(JailError::UnsafeMount(
            "Cannot mount root filesystem (/) into container".to_string(),
        ));
    }

    let home_dir = std::env::var("HOME")
        .map_err(|_| JailError::Config("HOME environment variable not set".to_string()))?;
    let home_path = std::path::PathBuf::from(&home_dir)
        .canonicalize()
        .map_err(JailError::Io)?;

    if source == home_path {
        return Err(JailError::UnsafeMount(format!(
            "Cannot mount entire home directory ({}) into container",
            home_path.display()
        )));
    }

    Ok(())
}

async fn run(command: Option<Commands>, verbose: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let _prefetch_handle = image_parallel::prefetch_common_layers(&cwd);

    match command {
        None => {
            let workspace_dir = jail_detection::get_git_root().unwrap_or_else(|| cwd.clone());
            let matching_jails = jail_detection::find_jails_for_directory(&workspace_dir).await?;

            let jail_name = if matching_jails.is_empty() {
                let base_name = cli::Commands::generate_jail_name(&workspace_dir);
                tracing::info!("No jail found for this directory, creating default jail...");
                let jail_name = format!("{base_name}__default");

                tracing::info!("Creating jail '{}'...", jail_name);
                let jail = create_default_jail(&jail_name, &workspace_dir, verbose).await?;
                jail.create().await?;
                tracing::info!("Jail '{}' created successfully", jail_name);

                jail_name
            } else if matching_jails.len() == 1 {
                let jail_name = matching_jails[0].clone();
                tracing::info!("Found single jail for this directory: '{}'", jail_name);
                jail_name
            } else {
                agent_commands::select_jail(&matching_jails)?
            };

            tracing::info!("Executing interactive shell in jail '{}'...", jail_name);
            let jail = jail::JailBuilder::new(jail_name.clone())
                .backend(config::BackendType::detect())
                .verbose(verbose)
                .build();

            jail.exec(&["/usr/bin/zsh".to_string()], true).await?;
        }
        Some(command) => match command {
            Commands::Create {
                name,
                backend,
                image,
                mount,
                port,
                env,
                no_network,
                host_network,
                memory,
                cpu,
                config,
                no_workspace,
                workspace_path,
                agent_configs,
                git_gpg,
                upgrade,
                layers,
                isolated,
                no_nix,
                no_block_host,
                podman,
            } => {
                let jail = if let Some(config_path) = config {
                    let config_str = tokio::fs::read_to_string(&config_path).await?;
                    let config: JailConfig = serde_json::from_str(&config_str)?;
                    jail::JailManager::new(config)
                } else {
                    let backend_type = if let Some(backend_str) = backend {
                        Commands::parse_backend(&backend_str).map_err(JailError::Config)?
                    } else {
                        config::BackendType::detect()
                    };

                    let jail_name = if let Some(name) = name {
                        cli::Commands::sanitize_jail_name(&name)
                    } else {
                        let cwd = std::env::current_dir()?;
                        let dir_name = cwd
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("default");
                        let generated_name = cli::Commands::sanitize_jail_name(dir_name);
                        tracing::info!(
                            "Auto-generated jail name from current directory: {}",
                            generated_name
                        );
                        generated_name
                    };

                    let mut builder = jail::JailBuilder::new(&jail_name)
                        .backend(backend_type)
                        .base_image(image);

                    builder = if host_network {
                        builder.host_network(true)
                    } else {
                        builder.network(!no_network, true)
                    };

                    builder = jail_setup::setup_default_environment(builder);

                    if !no_workspace {
                        let workspace_dir = jail_detection::get_git_root()
                            .unwrap_or_else(|| std::env::current_dir().unwrap());
                        jail_detection::validate_workspace_directory(&workspace_dir)?;
                        tracing::info!(
                            "Auto-mounting {} to {}",
                            workspace_dir.display(),
                            workspace_path
                        );
                        builder = builder.bind_mount(workspace_dir, workspace_path, false);
                    }

                    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
                    let home_path = std::path::PathBuf::from(&home);

                    builder = jail_setup::mount_agent_configs(
                        builder,
                        &home_path,
                        "",
                        false,
                        agent_configs,
                    );

                    if git_gpg {
                        let cwd = std::env::current_dir()?;
                        builder = git_gpg::setup_git_gpg_config(builder, &cwd, &home_path)?;
                    }

                    for mount_str in mount {
                        let mount = Commands::parse_mount(&mount_str).map_err(JailError::Config)?;
                        builder = builder.bind_mount(mount.source, mount.target, mount.readonly);
                    }

                    for port_str in port {
                        let port_mapping = Commands::parse_port(&port_str).map_err(JailError::Config)?;
                        builder = builder.port_mapping(
                            port_mapping.host_port,
                            port_mapping.container_port,
                            &port_mapping.protocol,
                        );
                    }

                    for env_str in env {
                        let (key, value) = Commands::parse_env(&env_str).map_err(JailError::Config)?;
                        builder = builder.env(key, value);
                    }

                    if let Some(mem) = memory {
                        builder = builder.memory_limit(mem);
                    }
                    if let Some(cpu_quota) = cpu {
                        builder = builder.cpu_quota(cpu_quota);
                    }

                    builder = builder.upgrade(upgrade);
                    builder = builder.force_layers(layers);
                    builder = builder.isolated(isolated);
                    builder = builder.verbose(verbose);
                    builder = builder.no_nix(no_nix);
                    builder = builder.block_host(!no_block_host);
                    builder = builder.podman_socket(podman);

                    builder.build()
                };

                jail.create().await?;

                if git_gpg {
                    let cwd = std::env::current_dir()?;
                    if let Err(e) = git_gpg::create_gitconfig_in_container(&cwd, &jail).await {
                        tracing::warn!("Failed to create .gitconfig in container: {}", e);
                    }
                }

                tracing::info!("Jail created: {}", jail.config().name);
            }

            Commands::Remove { name, force, volume } => {
                let jail_name = upgrade::resolve_jail_name(name).await?;

                if !force {
                    use std::io::{BufRead, Write};
                    print!("Remove jail '{jail_name}'? [y/N] ");
                    std::io::stdout().flush()?;
                    let stdin = std::io::stdin();
                    let mut line = String::new();
                    stdin.lock().read_line(&mut line)?;
                    if !line.trim().eq_ignore_ascii_case("y") {
                        tracing::info!("Aborted");
                        return Ok(());
                    }
                }

                let config = JailConfig {
                    name: jail_name.clone(),
                    ..Default::default()
                };
                let jail = jail::JailManager::new(config);
                jail.remove(volume).await?;

                tracing::info!("Jail removed: {}", jail_name);
            }

            Commands::Status { name } => {
                let jail_name = upgrade::resolve_jail_name(name).await?;
                let config = JailConfig {
                    name: jail_name.clone(),
                    ..Default::default()
                };
                let jail = jail::JailManager::new(config);
                let exists = jail.exists().await?;
                if exists {
                    println!("✓ Jail '{}' exists", jail_name);
                } else {
                    println!("✗ Jail '{}' does not exist", jail_name);
                }
            }

            Commands::Save { name, output } => {
                let jail_name = upgrade::resolve_jail_name(name).await?;
                let temp_config = JailConfig {
                    name: jail_name.clone(),
                    ..Default::default()
                };
                let jail = jail::JailManager::new(temp_config);

                if !jail.exists().await? {
                    return Err(JailError::NotFound(format!(
                        "Jail '{jail_name}' does not exist"
                    )));
                }

                let config = jail.inspect().await?;
                let json = serde_json::to_string_pretty(&config)?;
                tokio::fs::write(&output, json).await?;
                println!(
                    "✓ Configuration for jail '{}' saved to: {}",
                    jail_name,
                    output.display()
                );
                tracing::info!("Configuration saved to: {}", output.display());
            }

            Commands::Agents { common, subcommand } => {
                let agent = subcommand.to_agent();
                let args = subcommand.args();
                run_agent_command(agent, common, args, verbose).await?
            }

            Commands::List { current, backend } => {
                let backend_type = if let Some(backend_str) = backend {
                    Commands::parse_backend(&backend_str).map_err(JailError::Config)?
                } else {
                    config::BackendType::detect()
                };

                let temp_config = JailConfig {
                    name: "temp".to_string(),
                    backend: backend_type,
                    ..Default::default()
                };
                let backend = backend::create_backend(&temp_config);

                let all_jails = backend.list_all().await?;

                let jails = if current {
                    let cwd = std::env::current_dir()?;
                    let workspace_dir = jail_detection::get_git_root().unwrap_or(cwd);
                    let base_name = cli::Commands::generate_jail_name(&workspace_dir);
                    all_jails
                        .into_iter()
                        .filter(|name| name.starts_with(&base_name))
                        .collect::<Vec<_>>()
                } else {
                    all_jails
                };

                if jails.is_empty() {
                    if current {
                        println!("No jails found for current directory");
                    } else {
                        println!("No jails found");
                    }
                } else {
                    println!("Jails (backend: {backend_type:?}):");
                    for jail_name in &jails {
                        let agent_suffix = jail_detection::extract_agent_name(jail_name);
                        let config = JailConfig {
                            name: jail_name.clone(),
                            backend: backend_type,
                            ..Default::default()
                        };
                        let jail = jail::JailManager::new(config);
                        let status = if jail.exists().await? { "active" } else { "inactive" };
                        println!("  {jail_name} [{status}] ({agent_suffix})");
                    }
                    println!("\nTotal: {} jail(s)", jails.len());
                }
            }

            Commands::CleanAll { backend, force, volume } => {
                let backends = if let Some(backend_str) = backend {
                    vec![Commands::parse_backend(&backend_str).map_err(JailError::Config)?]
                } else {
                    let available = config::BackendType::all_available();
                    if available.is_empty() {
                        tracing::warn!("No backends are available on this system");
                        return Ok(());
                    }
                    available
                };

                for backend_type in backends {
                    tracing::info!(
                        "Cleaning all jail-ai containers for backend: {:?}",
                        backend_type
                    );

                    let temp_config = JailConfig {
                        name: "temp".to_string(),
                        backend: backend_type,
                        ..Default::default()
                    };
                    let temp_jail = jail::JailManager::new(temp_config);

                    let backend = backend::create_backend(temp_jail.config());
                    let jails = backend.list_all().await?;

                    if jails.is_empty() {
                        tracing::info!("No jail-ai containers found for backend {:?}", backend_type);
                        continue;
                    }

                    tracing::info!(
                        "Found {} jail-ai container(s) for backend {:?}",
                        jails.len(),
                        backend_type
                    );

                    if !force {
                        use std::io::{BufRead, Write};
                        println!("Containers to be removed:");
                        for jail_name in &jails {
                            println!("  - {jail_name}");
                        }
                        print!("Remove all {} container(s)? [y/N] ", jails.len());
                        std::io::stdout().flush()?;
                        let stdin = std::io::stdin();
                        let mut line = String::new();
                        stdin.lock().read_line(&mut line)?;
                        if !line.trim().eq_ignore_ascii_case("y") {
                            tracing::info!("Aborted");
                            continue;
                        }
                    }

                    for jail_name in jails {
                        tracing::info!("Removing jail: {}", jail_name);
                        let config = JailConfig {
                            name: jail_name.clone(),
                            backend: backend_type,
                            ..Default::default()
                        };
                        let jail = jail::JailManager::new(config);

                        if let Err(e) = jail.remove(volume).await {
                            tracing::error!("Failed to remove jail {}: {}", jail_name, e);
                        } else {
                            tracing::info!("Successfully removed jail: {}", jail_name);
                        }
                    }
                }

                tracing::info!("Clean-all operation completed");
            }

            Commands::Upgrade { name, image, force, all } => {
                if all {
                    upgrade::upgrade_all_jails(image, force, verbose).await?;
                } else {
                    let jail_name = upgrade::resolve_jail_name(name).await?;
                    upgrade::upgrade_single_jail(&jail_name, image, force, verbose).await?;
                }
            }

            Commands::Completions { shell } => {
                cli::Cli::generate_completions(shell);
            }
        },
    }

    Ok(())
}

async fn run_agent_command(
    agent: agents::Agent,
    common: cli::AgentCommandOptions,
    args: Vec<String>,
    verbose: bool,
) -> Result<()> {
    agent_commands::run_ai_agent_command(
        agent.command_name(),
        agent_commands::AgentCommandParams {
            backend: common.backend,
            image: common.image,
            mount: common.mount,
            port: common.port,
            env: common.env,
            no_network: common.no_network,
            host_network: common.host_network,
            memory: common.memory,
            cpu: common.cpu,
            no_workspace: common.no_workspace,
            workspace_path: common.workspace_path,
            agent_configs: common.agent_configs,
            git_gpg: common.git_gpg,
            upgrade: common.upgrade,
            force_layers: common.layers,
            cloud: common.cloud,
            shell: common.shell,
            isolated: common.isolated,
            verbose,
            auth: common.auth,
            no_nix: common.no_nix,
            no_block_host: common.no_block_host,
            podman: common.podman,
            tui: common.tui,
            args,
        },
    )
    .await
}

async fn create_default_jail(
    name: &str,
    workspace: &std::path::Path,
    verbose: bool,
) -> Result<jail::JailManager> {
    let backend_type = config::BackendType::detect();

    let mut builder = jail::JailBuilder::new(name)
        .backend(backend_type)
        .base_image(image::DEFAULT_IMAGE_NAME)
        .network(true, true)
        .verbose(verbose);

    builder = jail_setup::setup_default_environment(builder);

    let workspace_dir = jail_detection::get_git_root().unwrap_or(workspace.to_path_buf());
    jail_detection::validate_workspace_directory(&workspace_dir)?;

    tracing::info!("Auto-mounting {} to /workspace", workspace_dir.display());
    builder = builder.bind_mount(workspace_dir, "/workspace", false);

    Ok(builder.build())
}
