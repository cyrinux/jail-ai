mod backend;
mod cli;
mod config;
mod error;
mod image;
mod jail;

use clap::Parser;
use cli::{Cli, Commands};
use config::JailConfig;
use error::JailError;
use jail::JailBuilder;
use std::path::PathBuf;
use tracing::{error, info, warn};
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
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| filter.into()),
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
            // Default behavior: auto-init and exec based on workspace (git root if available)
            let cwd = std::env::current_dir()?;
            let workspace_dir = get_git_root().unwrap_or(cwd.clone());

            // Find all jails for this directory
            let matching_jails = find_jails_for_directory(&workspace_dir).await?;

            let jail_name = if matching_jails.is_empty() {
                // No jails exist, create a default one
                let base_name = cli::Commands::generate_jail_name(&workspace_dir);
                info!("No jail found for this directory, creating default jail...");
                let jail_name = format!("{}-default", base_name);

                info!("Creating jail '{}'...", jail_name);
                let jail = create_default_jail(&jail_name, &workspace_dir).await?;
                jail.create().await?;
                info!("Jail '{}' created successfully", jail_name);

                jail_name
            } else if matching_jails.len() == 1 {
                // Only one jail exists, use it
                let jail_name = matching_jails[0].clone();
                info!("Found single jail for this directory: '{}'", jail_name);
                jail_name
            } else {
                // Multiple jails exist, ask user to choose
                select_jail(&matching_jails)?
            };

            // Exec into jail with interactive shell
            info!("Executing interactive shell in jail '{}'...", jail_name);
            let jail = JailBuilder::new(jail_name.clone())
                .backend(config::BackendType::detect())
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
                claude_dir,
                copilot_dir,
                cursor_dir,
                agent_configs,
                git_gpg,
            } => {
                let jail = if let Some(config_path) = config {
                    // Load from config file
                    let config_str = tokio::fs::read_to_string(&config_path).await?;
                    let config: JailConfig = serde_json::from_str(&config_str)?;
                    jail::JailManager::new(config)
                } else {
                    // Build from CLI args
                    let backend_type = if let Some(backend_str) = backend {
                        Commands::parse_backend(&backend_str).map_err(error::JailError::Config)?
                    } else {
                        config::BackendType::detect()
                    };

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
                        info!(
                            "Auto-generated jail name from current directory: {}",
                            generated_name
                        );
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

                    // Inherit TERM from host
                    if let Ok(term) = std::env::var("TERM") {
                        builder = builder.env("TERM", term);
                    }

                    // Auto-mount workspace (git root if available, otherwise current directory)
                    if !no_workspace {
                        let workspace_dir =
                            get_git_root().unwrap_or_else(|| std::env::current_dir().unwrap());
                        info!(
                            "Auto-mounting {} to {}",
                            workspace_dir.display(),
                            workspace_path
                        );
                        builder = builder.bind_mount(workspace_dir, workspace_path, false);
                    }

                    // Opt-in: Mount agent config directories
                    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
                    let home_path = std::path::PathBuf::from(&home);

                    // Opt-in: Mount entire ~/.claude directory
                    if claude_dir || agent_configs {
                        let claude_config = home_path.join(".claude");
                        if claude_config.exists() {
                            info!(
                                "Mounting {} to /home/agent/.claude",
                                claude_config.display()
                            );
                            builder =
                                builder.bind_mount(claude_config, "/home/agent/.claude", false);
                        }
                    }

                    // Opt-in: Mount ~/.config/.copilot for GitHub Copilot
                    if copilot_dir || agent_configs {
                        let copilot_config = home_path.join(".config").join(".copilot");
                        if copilot_config.exists() {
                            info!(
                                "Mounting {} to /home/agent/.config/.copilot",
                                copilot_config.display()
                            );
                            builder = builder.bind_mount(
                                copilot_config,
                                "/home/agent/.config/.copilot",
                                false,
                            );
                        }
                    }

                    // Opt-in: Mount ~/.cursor for Cursor Agent
                    if cursor_dir || agent_configs {
                        let cursor_config = home_path.join(".cursor");
                        if cursor_config.exists() {
                            info!(
                                "Mounting {} to /home/agent/.cursor",
                                cursor_config.display()
                            );
                            builder =
                                builder.bind_mount(cursor_config, "/home/agent/.cursor", false);
                        }
                    }

                    // Opt-in: Git and GPG configuration
                    if git_gpg {
                        let cwd = std::env::current_dir()?;
                        let git_config_path = cwd.join(".git").join("config");
                        if git_config_path.exists() {
                            info!(
                                "Mounting git config {} to /home/agent/.gitconfig",
                                git_config_path.display()
                            );
                            builder = builder.bind_mount(
                                git_config_path,
                                "/home/agent/.gitconfig",
                                false,
                            );
                        } else {
                            // If no local git config, try to get global git config and set env vars
                            if let Ok(git_name) = std::process::Command::new("git")
                                .args(["config", "--global", "user.name"])
                                .output()
                            {
                                if git_name.status.success() {
                                    let name = String::from_utf8_lossy(&git_name.stdout)
                                        .trim()
                                        .to_string();
                                    if !name.is_empty() {
                                        builder = builder.env("GIT_AUTHOR_NAME", &name);
                                        builder = builder.env("GIT_COMMITTER_NAME", &name);
                                    }
                                }
                            }

                            if let Ok(git_email) = std::process::Command::new("git")
                                .args(["config", "--global", "user.email"])
                                .output()
                            {
                                if git_email.status.success() {
                                    let email = String::from_utf8_lossy(&git_email.stdout)
                                        .trim()
                                        .to_string();
                                    if !email.is_empty() {
                                        builder = builder.env("GIT_AUTHOR_EMAIL", &email);
                                        builder = builder.env("GIT_COMMITTER_EMAIL", &email);
                                    }
                                }
                            }

                            if let Ok(git_signing_key) = std::process::Command::new("git")
                                .args(["config", "--global", "user.signingkey"])
                                .output()
                            {
                                if git_signing_key.status.success() {
                                    let signing_key =
                                        String::from_utf8_lossy(&git_signing_key.stdout)
                                            .trim()
                                            .to_string();
                                    if !signing_key.is_empty() {
                                        builder = builder.env("GIT_SIGNING_KEY", &signing_key);
                                    }
                                }
                            }
                        }

                        // Mount GPG configuration
                        let gpg_dir = home_path.join(".gnupg");
                        if gpg_dir.exists() {
                            info!(
                                "Mounting GPG config {} to /home/agent/.gnupg",
                                gpg_dir.display()
                            );
                            builder = builder.bind_mount(gpg_dir, "/home/agent/.gnupg", false);
                        }
                    }

                    // Parse mounts
                    for mount_str in mount {
                        let mount =
                            Commands::parse_mount(&mount_str).map_err(error::JailError::Config)?;
                        builder = builder.bind_mount(mount.source, mount.target, mount.readonly);
                    }

                    // Parse environment variables
                    for env_str in env {
                        let (key, value) =
                            Commands::parse_env(&env_str).map_err(error::JailError::Config)?;
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
                let jail_name = resolve_jail_name(name)?;
                let config = JailConfig {
                    name: jail_name.clone(),
                    ..Default::default()
                };
                let jail = jail::JailManager::new(config);
                jail.start().await?;
                info!("Jail started: {}", jail_name);
            }

            Commands::Stop { name } => {
                let jail_name = resolve_jail_name(name)?;
                let config = JailConfig {
                    name: jail_name.clone(),
                    ..Default::default()
                };
                let jail = jail::JailManager::new(config);
                jail.stop().await?;
                info!("Jail stopped: {}", jail_name);
            }

            Commands::Remove { name, force } => {
                let jail_name = resolve_jail_name(name)?;

                if !force {
                    use std::io::{self, BufRead, Write};
                    print!("Remove jail '{}'? [y/N] ", jail_name);
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
                    name: jail_name.clone(),
                    ..Default::default()
                };
                let jail = jail::JailManager::new(config);
                jail.remove().await?;
                info!("Jail removed: {}", jail_name);
            }

            Commands::Exec {
                name,
                command,
                interactive,
            } => {
                if command.is_empty() {
                    return Err(error::JailError::Config("No command specified".to_string()));
                }

                let jail_name = resolve_jail_name(name)?;
                let config = JailConfig {
                    name: jail_name,
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
                let jail_name = resolve_jail_name(name)?;
                let config = JailConfig {
                    name: jail_name.clone(),
                    ..Default::default()
                };
                let jail = jail::JailManager::new(config);
                let exists = jail.exists().await?;
                if exists {
                    info!("Jail '{}' exists", jail_name);
                } else {
                    info!("Jail '{}' does not exist", jail_name);
                }
            }

            Commands::Save { name, output } => {
                let jail_name = resolve_jail_name(name)?;
                let config = JailConfig {
                    name: jail_name,
                    ..Default::default()
                };
                let json = serde_json::to_string_pretty(&config)?;
                tokio::fs::write(&output, json).await?;
                info!("Configuration saved to: {}", output.display());
            }

            Commands::Claude {
                backend,
                image,
                mount,
                env,
                no_network,
                memory,
                cpu,
                no_workspace,
                workspace_path,
                claude_dir,
                copilot_dir,
                cursor_dir,
                agent_configs,
                git_gpg,
                args,
            } => {
                run_ai_agent_command(
                    "claude",
                    AgentCommandParams {
                        backend,
                        image,
                        mount,
                        env,
                        no_network,
                        memory,
                        cpu,
                        no_workspace,
                        workspace_path,
                        claude_dir,
                        copilot_dir,
                        cursor_dir,
                        agent_configs,
                        git_gpg,
                        args,
                    },
                )
                .await?;
            }

            Commands::Copilot {
                backend,
                image,
                mount,
                env,
                no_network,
                memory,
                cpu,
                no_workspace,
                workspace_path,
                claude_dir,
                copilot_dir,
                cursor_dir,
                agent_configs,
                git_gpg,
                args,
            } => {
                run_ai_agent_command(
                    "copilot",
                    AgentCommandParams {
                        backend,
                        image,
                        mount,
                        env,
                        no_network,
                        memory,
                        cpu,
                        no_workspace,
                        workspace_path,
                        claude_dir,
                        copilot_dir,
                        cursor_dir,
                        agent_configs,
                        git_gpg,
                        args,
                    },
                )
                .await?;
            }

            Commands::Cursor {
                backend,
                image,
                mount,
                env,
                no_network,
                memory,
                cpu,
                no_workspace,
                workspace_path,
                claude_dir,
                copilot_dir,
                cursor_dir,
                agent_configs,
                git_gpg,
                args,
            } => {
                run_ai_agent_command(
                    "cursor-agent",
                    AgentCommandParams {
                        backend,
                        image,
                        mount,
                        env,
                        no_network,
                        memory,
                        cpu,
                        no_workspace,
                        workspace_path,
                        claude_dir,
                        copilot_dir,
                        cursor_dir,
                        agent_configs,
                        git_gpg,
                        args,
                    },
                )
                .await?;
            }

            Commands::List { current, backend } => {
                // Determine backend to use
                let backend_type = if let Some(backend_str) = backend {
                    Commands::parse_backend(&backend_str).map_err(error::JailError::Config)?
                } else {
                    config::BackendType::detect()
                };

                let temp_config = JailConfig {
                    name: "temp".to_string(),
                    backend: backend_type,
                    ..Default::default()
                };
                let backend = backend::create_backend(&temp_config);

                // Get all jails
                let all_jails = backend.list_all().await?;

                // Filter by current directory if requested
                let jails = if current {
                    let cwd = std::env::current_dir()?;
                    let workspace_dir = get_git_root().unwrap_or(cwd);
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
                    println!("Jails (backend: {:?}):", backend_type);
                    for jail_name in &jails {
                        // Extract agent name from jail name
                        let agent_suffix = jail_name.split('-').next_back().unwrap_or("unknown");

                        // Check if jail is running
                        let config = JailConfig {
                            name: jail_name.clone(),
                            backend: backend_type,
                            ..Default::default()
                        };
                        let jail = jail::JailManager::new(config);
                        let status = if jail.exists().await? {
                            "active"
                        } else {
                            "inactive"
                        };

                        println!("  {} [{}] ({})", jail_name, status, agent_suffix);
                    }
                    println!("\nTotal: {} jail(s)", jails.len());
                }
            }

            Commands::Join { shell } => {
                let jail_name = auto_detect_jail_name()?;

                info!("Joining jail: {}", jail_name);

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

            Commands::CleanAll { backend, force } => {
                // Determine which backends to clean
                let backends = if let Some(backend_str) = backend {
                    vec![Commands::parse_backend(&backend_str).map_err(error::JailError::Config)?]
                } else {
                    // Clean all available backends by default
                    let available = config::BackendType::all_available();
                    if available.is_empty() {
                        warn!("No backends are available on this system");
                        return Ok(());
                    }
                    available
                };

                for backend_type in backends {
                    info!(
                        "Cleaning all jail-ai containers for backend: {:?}",
                        backend_type
                    );

                    // Create a temporary config with the backend type
                    let temp_config = JailConfig {
                        name: "temp".to_string(),
                        backend: backend_type,
                        ..Default::default()
                    };
                    let temp_jail = jail::JailManager::new(temp_config);

                    // Get list of all jails
                    let backend = backend::create_backend(temp_jail.config());
                    let jails = backend.list_all().await?;

                    if jails.is_empty() {
                        info!("No jail-ai containers found for backend {:?}", backend_type);
                        continue;
                    }

                    info!(
                        "Found {} jail-ai container(s) for backend {:?}",
                        jails.len(),
                        backend_type
                    );

                    // Ask for confirmation unless force is specified
                    if !force {
                        use std::io::{self, BufRead, Write};
                        println!("Containers to be removed:");
                        for jail_name in &jails {
                            println!("  - {}", jail_name);
                        }
                        print!("Remove all {} container(s)? [y/N] ", jails.len());
                        io::stdout().flush()?;
                        let stdin = io::stdin();
                        let mut line = String::new();
                        stdin.lock().read_line(&mut line)?;
                        if !line.trim().eq_ignore_ascii_case("y") {
                            info!("Aborted");
                            continue;
                        }
                    }

                    // Remove each jail
                    for jail_name in jails {
                        info!("Removing jail: {}", jail_name);
                        let config = JailConfig {
                            name: jail_name.clone(),
                            backend: backend_type,
                            ..Default::default()
                        };
                        let jail = jail::JailManager::new(config);

                        if let Err(e) = jail.remove().await {
                            error!("Failed to remove jail {}: {}", jail_name, e);
                        } else {
                            info!("Successfully removed jail: {}", jail_name);
                        }
                    }
                }

                info!("Clean-all operation completed");
            }
        },
    }

    Ok(())
}

/// Auto-detect jail name from current directory (or git root if available)
fn auto_detect_jail_name() -> error::Result<String> {
    let cwd = std::env::current_dir()?;
    let workspace_dir = get_git_root().unwrap_or(cwd);
    let jail_name = cli::Commands::generate_jail_name(&workspace_dir);
    info!("Auto-detected jail name from workspace: {}", jail_name);
    Ok(jail_name)
}

/// Resolve jail name: use provided name or auto-detect from current directory
fn resolve_jail_name(name: Option<String>) -> error::Result<String> {
    if let Some(name) = name {
        Ok(name)
    } else {
        auto_detect_jail_name()
    }
}

/// Get the git root directory if the current directory is within a git repository
fn get_git_root() -> Option<PathBuf> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let git_root = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !git_root.is_empty() {
                let path = PathBuf::from(git_root);
                if path.exists() {
                    info!("Found git root: {}", path.display());
                    return Some(path);
                }
            }
        }
        _ => {
            // Not a git repository or git command failed
        }
    }

    None
}

/// Get the host's timezone
fn get_host_timezone() -> Option<String> {
    // Try TZ environment variable first
    if let Ok(tz) = std::env::var("TZ") {
        if !tz.is_empty() {
            info!("Using timezone from TZ env var: {}", tz);
            return Some(tz);
        }
    }

    // Try timedatectl (systemd-based systems)
    if let Ok(output) = std::process::Command::new("timedatectl")
        .arg("show")
        .arg("--property=Timezone")
        .arg("--value")
        .output()
    {
        if output.status.success() {
            let tz = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !tz.is_empty() && tz != "n/a" {
                info!("Using timezone from timedatectl: {}", tz);
                return Some(tz);
            }
        }
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
    if let Ok(link) = std::fs::read_link("/etc/localtime") {
        if let Some(tz) = link.to_str() {
            // Extract timezone from paths like:
            // - /usr/share/zoneinfo/Europe/Paris
            // - /run/current-system/sw/share/zoneinfo/Europe/Paris (NixOS)
            // - ../usr/share/zoneinfo/Europe/Paris (relative symlinks)
            for prefix in [
                "/usr/share/zoneinfo/",
                "/run/current-system/sw/share/zoneinfo/",
                "../usr/share/zoneinfo/",
            ] {
                if let Some(tz_name) = tz.strip_prefix(prefix) {
                    info!("Using timezone from /etc/localtime: {}", tz_name);
                    return Some(tz_name.to_string());
                }
            }
            // Try extracting from any path containing "zoneinfo/"
            if let Some(pos) = tz.find("zoneinfo/") {
                let tz_name = &tz[pos + "zoneinfo/".len()..];
                if !tz_name.is_empty() {
                    info!(
                        "Using timezone from /etc/localtime (extracted): {}",
                        tz_name
                    );
                    return Some(tz_name.to_string());
                }
            }
        }
    }

    warn!("Could not determine host timezone, container will use UTC. Try setting TZ environment variable or ensure timedatectl is available.");
    None
}

/// Helper function to create a jail with default configuration
async fn create_default_jail(
    name: &str,
    workspace: &std::path::Path,
) -> error::Result<jail::JailManager> {
    let backend_type = config::BackendType::detect();

    let mut builder = JailBuilder::new(name)
        .backend(backend_type)
        .base_image(image::DEFAULT_IMAGE_NAME)
        .network(true, true);

    // Set timezone from host
    if let Some(tz) = get_host_timezone() {
        builder = builder.env("TZ", tz);
    }

    // Inherit TERM from host
    if let Ok(term) = std::env::var("TERM") {
        builder = builder.env("TERM", term);
    }

    // Auto-mount workspace (git root if available, otherwise provided workspace)
    let workspace_dir = get_git_root().unwrap_or(workspace.to_path_buf());
    info!("Auto-mounting {} to /workspace", workspace_dir.display());
    builder = builder.bind_mount(workspace_dir, "/workspace", false);

    Ok(builder.build())
}

/// Find all jails matching the current directory pattern
async fn find_jails_for_directory(workspace_dir: &std::path::Path) -> error::Result<Vec<String>> {
    let base_name = cli::Commands::generate_jail_name(workspace_dir);
    let backend_type = config::BackendType::detect();

    // Create a temporary config just to access the backend
    let temp_config = JailConfig {
        name: "temp".to_string(),
        backend: backend_type,
        ..Default::default()
    };
    let backend = backend::create_backend(&temp_config);

    // List all jails
    let all_jails = backend.list_all().await?;

    // Filter jails that match the base pattern (jail-{project}-{hash}-)
    let matching_jails: Vec<String> = all_jails
        .into_iter()
        .filter(|name| name.starts_with(&base_name) && name.len() > base_name.len())
        .collect();

    Ok(matching_jails)
}

/// Prompt user to select a jail from a list
fn select_jail(jails: &[String]) -> error::Result<String> {
    use std::io::{self, Write};

    println!("Multiple jails found for this directory:");
    for (i, jail) in jails.iter().enumerate() {
        // Extract agent name from jail name
        let agent_name = jail.split('-').next_back().unwrap_or("unknown");
        println!("  {}. {} (agent: {})", i + 1, jail, agent_name);
    }

    print!("Select a jail (1-{}): ", jails.len());
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| JailError::Config(format!("Failed to read input: {}", e)))?;

    let selection: usize = input
        .trim()
        .parse()
        .map_err(|_| JailError::Config("Invalid selection".to_string()))?;

    if selection < 1 || selection > jails.len() {
        return Err(JailError::Config("Selection out of range".to_string()));
    }

    Ok(jails[selection - 1].clone())
}

/// Parameters for AI agent commands
struct AgentCommandParams {
    backend: Option<String>,
    image: String,
    mount: Vec<String>,
    env: Vec<String>,
    no_network: bool,
    memory: Option<u64>,
    cpu: Option<u32>,
    no_workspace: bool,
    workspace_path: String,
    claude_dir: bool,
    copilot_dir: bool,
    cursor_dir: bool,
    agent_configs: bool,
    git_gpg: bool,
    args: Vec<String>,
}

/// Helper function to run AI agent commands (claude, copilot, cursor-agent)
async fn run_ai_agent_command(
    agent_command: &str,
    params: AgentCommandParams,
) -> error::Result<()> {
    let cwd = std::env::current_dir()?;
    let base_name = auto_detect_jail_name()?;
    let agent_suffix = cli::Commands::sanitize_jail_name(agent_command);
    let jail_name = format!("{}-{}", base_name, agent_suffix);

    info!("Using jail: {} for agent: {}", jail_name, agent_command);

    // Create jail if it doesn't exist
    let temp_config = JailConfig {
        name: jail_name.clone(),
        ..Default::default()
    };
    let temp_jail = jail::JailManager::new(temp_config);

    if !temp_jail.exists().await? {
        info!("Creating new jail: {}", jail_name);

        let backend_type = if let Some(backend_str) = params.backend {
            Commands::parse_backend(&backend_str).map_err(error::JailError::Config)?
        } else {
            config::BackendType::detect()
        };

        let mut builder = JailBuilder::new(&jail_name)
            .backend(backend_type)
            .base_image(params.image)
            .network(!params.no_network, true);

        // Set timezone from host
        if let Some(tz) = get_host_timezone() {
            builder = builder.env("TZ", tz);
        }

        // Inherit TERM from host
        if let Ok(term) = std::env::var("TERM") {
            builder = builder.env("TERM", term);
        }

        // Auto-mount workspace (git root if available, otherwise current directory)
        if !params.no_workspace {
            let workspace_dir = get_git_root().unwrap_or(cwd.clone());
            info!(
                "Auto-mounting {} to {}",
                workspace_dir.display(),
                params.workspace_path
            );
            builder = builder.bind_mount(workspace_dir, params.workspace_path, false);
        }

        // Auto-mount minimal auth files (agent-specific)
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let home_path = std::path::PathBuf::from(&home);

        // Mount ~/.claude/.credentials.json only for Claude agent
        if agent_command == "claude" {
            let claude_creds = home_path.join(".claude").join(".credentials.json");
            if claude_creds.exists() {
                info!(
                    "Auto-mounting {} to /home/agent/.claude/.credentials.json",
                    claude_creds.display()
                );
                builder = builder.bind_mount(
                    claude_creds,
                    "/home/agent/.claude/.credentials.json",
                    false,
                );
            }
        }

        // Opt-in: Mount entire ~/.claude directory
        if params.claude_dir || params.agent_configs {
            let claude_config = home_path.join(".claude");
            if claude_config.exists() {
                info!(
                    "Mounting {} to /home/agent/.claude",
                    claude_config.display()
                );
                builder = builder.bind_mount(claude_config, "/home/agent/.claude", false);
            }
        }

        // Opt-in: Mount ~/.config/.copilot for GitHub Copilot
        if params.copilot_dir || params.agent_configs {
            let copilot_config = home_path.join(".config").join(".copilot");
            if copilot_config.exists() {
                info!(
                    "Mounting {} to /home/agent/.config/.copilot",
                    copilot_config.display()
                );
                builder = builder.bind_mount(copilot_config, "/home/agent/.config/.copilot", false);
            }
        }

        // Opt-in: Mount ~/.cursor for Cursor Agent
        if params.cursor_dir || params.agent_configs {
            let cursor_config = home_path.join(".cursor");
            if cursor_config.exists() {
                info!(
                    "Mounting {} to /home/agent/.cursor",
                    cursor_config.display()
                );
                builder = builder.bind_mount(cursor_config, "/home/agent/.cursor", false);
            }
        }

        // Opt-in: Git and GPG configuration
        if params.git_gpg {
            let git_config_path = cwd.join(".git").join("config");
            if git_config_path.exists() {
                info!(
                    "Mounting git config {} to /home/agent/.gitconfig",
                    git_config_path.display()
                );
                builder = builder.bind_mount(git_config_path, "/home/agent/.gitconfig", false);
            } else {
                // If no local git config, try to get global git config and set environment variables
                if let Ok(git_name) = std::process::Command::new("git")
                    .args(["config", "--global", "user.name"])
                    .output()
                {
                    if git_name.status.success() {
                        let name = String::from_utf8_lossy(&git_name.stdout).trim().to_string();
                        if !name.is_empty() {
                            builder = builder.env("GIT_AUTHOR_NAME", &name);
                            builder = builder.env("GIT_COMMITTER_NAME", &name);
                        }
                    }
                }

                if let Ok(git_email) = std::process::Command::new("git")
                    .args(["config", "--global", "user.email"])
                    .output()
                {
                    if git_email.status.success() {
                        let email = String::from_utf8_lossy(&git_email.stdout)
                            .trim()
                            .to_string();
                        if !email.is_empty() {
                            builder = builder.env("GIT_AUTHOR_EMAIL", &email);
                            builder = builder.env("GIT_COMMITTER_EMAIL", &email);
                        }
                    }
                }

                if let Ok(git_signing_key) = std::process::Command::new("git")
                    .args(["config", "--global", "user.signingkey"])
                    .output()
                {
                    if git_signing_key.status.success() {
                        let signing_key = String::from_utf8_lossy(&git_signing_key.stdout)
                            .trim()
                            .to_string();
                        if !signing_key.is_empty() {
                            builder = builder.env("GIT_SIGNING_KEY", &signing_key);
                        }
                    }
                }
            }

            // Mount GPG configuration
            let gpg_dir = home_path.join(".gnupg");
            if gpg_dir.exists() {
                info!(
                    "Mounting GPG config {} to /home/agent/.gnupg",
                    gpg_dir.display()
                );
                builder = builder.bind_mount(gpg_dir, "/home/agent/.gnupg", false);
            }
        }

        // Parse mounts
        for mount_str in params.mount {
            let mount = Commands::parse_mount(&mount_str).map_err(error::JailError::Config)?;
            builder = builder.bind_mount(mount.source, mount.target, mount.readonly);
        }

        // Parse environment variables
        for env_str in params.env {
            let (key, value) = Commands::parse_env(&env_str).map_err(error::JailError::Config)?;
            builder = builder.env(key, value);
        }

        // Set resource limits
        if let Some(mem) = params.memory {
            builder = builder.memory_limit(mem);
        }
        if let Some(cpu_quota) = params.cpu {
            builder = builder.cpu_quota(cpu_quota);
        }

        let jail = builder.build();
        jail.create().await?;
    }

    // Execute AI agent command
    let jail = JailBuilder::new(&jail_name)
        .backend(config::BackendType::detect())
        .build();
    let mut command = vec![agent_command.to_string()];
    command.extend(params.args);

    let output = jail.exec(&command, true).await?;
    if !output.is_empty() {
        print!("{}", output);
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
