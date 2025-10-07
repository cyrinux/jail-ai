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
                gemini_dir,
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

                    let mut builder = JailBuilder::new(&jail_name)
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
                    } else {
                        // If not mounting full .claude directory, mount minimal auth files
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

                    // Opt-in: Mount ~/.config/gemini for Gemini CLI
                    if gemini_dir || agent_configs {
                        let gemini_config = home_path.join(".config").join("gemini");
                        if gemini_config.exists() {
                            info!(
                                "Mounting {} to /home/agent/.config/gemini",
                                gemini_config.display()
                            );
                            builder = builder.bind_mount(
                                gemini_config,
                                "/home/agent/.config/gemini",
                                false,
                            );
                        }
                    }

                    // Opt-in: GPG configuration
                    if git_gpg {
                        // Prepare and mount GPG configuration directory
                        let gpg_dir = home_path.join(".gnupg");
                        if gpg_dir.exists() {
                            match prepare_gpg_config(&gpg_dir) {
                                Ok((temp_gpg_dir, sockets)) => {
                                    info!(
                                        "Mounting prepared GPG config {} to /home/agent/.gnupg",
                                        temp_gpg_dir.display()
                                    );
                                    builder = builder.bind_mount(&temp_gpg_dir, "/home/agent/.gnupg", false);

                                    // Mount GPG agent sockets for YubiKey and smartcard support
                                    for socket_path in sockets {
                                        if let Some(socket_name) = socket_path.file_name() {
                                            let socket_name_str = socket_name.to_string_lossy();
                                            let target = format!("/home/agent/.gnupg/{}", socket_name_str);
                                            info!(
                                                "Mounting GPG socket {} to {}",
                                                socket_path.display(),
                                                target
                                            );
                                            builder = builder.bind_mount(socket_path, target, false);
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to prepare GPG config: {}", e);
                                }
                            }
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

                // Create .claude.json file inside the container
                let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
                let home_path = std::path::PathBuf::from(&home);
                if let Err(e) = create_claude_json_in_container(&home_path, &jail).await {
                    warn!("Failed to create .claude.json in container: {}", e);
                }

                // Create .gitconfig file inside the container if git_gpg is enabled
                if git_gpg {
                    let cwd = std::env::current_dir()?;
                    if let Err(e) = create_gitconfig_in_container(&cwd, &jail).await {
                        warn!("Failed to create .gitconfig in container: {}", e);
                    }
                }

                info!("Jail created: {}", jail.config().name);
            }

            Commands::Remove { name, force, volume } => {
                let jail_name = resolve_jail_name(name).await?;

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
                jail.remove(volume).await?;

                info!("Jail removed: {}", jail_name);
            }

            Commands::Status { name } => {
                let jail_name = resolve_jail_name(name).await?;
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
                let jail_name = resolve_jail_name(name).await?;

                // Create a temporary jail manager to inspect the actual configuration
                let temp_config = JailConfig {
                    name: jail_name.clone(),
                    ..Default::default()
                };
                let jail = jail::JailManager::new(temp_config);

                // Check if jail exists
                if !jail.exists().await? {
                    return Err(error::JailError::NotFound(format!(
                        "Jail '{}' does not exist",
                        jail_name
                    )));
                }

                // Inspect the jail to get its actual configuration
                let config = jail.inspect().await?;

                let json = serde_json::to_string_pretty(&config)?;
                tokio::fs::write(&output, json).await?;
                println!("✓ Configuration for jail '{}' saved to: {}", jail_name, output.display());
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
                gemini_dir,
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
                        gemini_dir,
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
                gemini_dir,
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
                        gemini_dir,
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
                gemini_dir: _,
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
                        gemini_dir: false,
                        agent_configs,
                        git_gpg,
                        args,
                    },
                )
                .await?;
            }

            Commands::Gemini {
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
                gemini_dir,
                agent_configs,
                git_gpg,
                args,
            } => {
                run_ai_agent_command(
                    "gemini",
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
                        gemini_dir,
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

            Commands::CleanAll { backend, force, volume } => {
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

                        if let Err(e) = jail.remove(volume).await {
                            error!("Failed to remove jail {}: {}", jail_name, e);
                        } else {
                            info!("Successfully removed jail: {}", jail_name);
                        }
                    }
                }

                info!("Clean-all operation completed");
            }

            Commands::Upgrade { name, image, force, all } => {
                if all {
                    // Upgrade all jails
                    upgrade_all_jails(image, force).await?;
                } else {
                    // Upgrade single jail
                    let jail_name = resolve_jail_name(name).await?;
                    upgrade_single_jail(&jail_name, image, force).await?;
                }
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
async fn resolve_jail_name(name: Option<String>) -> error::Result<String> {
    if let Some(name) = name {
        Ok(name)
    } else {
        // Auto-detect: find all matching jails for this directory
        let cwd = std::env::current_dir()?;
        let workspace_dir = get_git_root().unwrap_or(cwd);
        let matching_jails = find_jails_for_directory(&workspace_dir).await?;

        let jail_name = if matching_jails.is_empty() {
            return Err(error::JailError::Config(
                "No jails found for this directory. Create one first.".to_string(),
            ));
        } else if matching_jails.len() == 1 {
            // Only one jail exists, use it
            matching_jails[0].clone()
        } else {
            // Multiple jails exist, ask user to choose
            select_jail(&matching_jails)?
        };

        info!("Auto-detected jail: {}", jail_name);
        Ok(jail_name)
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

/// Get git config value with fallback to global and system config
/// First tries project config, then global config, then system config
fn get_git_config(key: &str, cwd: &std::path::Path) -> Option<String> {
    use tracing::debug;

    // Try project-specific config first (local to the repository)
    // Use --get-all to handle duplicate entries and take the last one
    if let Ok(output) = std::process::Command::new("git")
        .current_dir(cwd)
        .args(["config", "--local", "--get-all", key])
        .output()
    {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            // Get the last non-empty line (git uses last value when there are duplicates)
            if let Some(value) = output_str.lines().filter(|l| !l.trim().is_empty()).next_back() {
                let value = value.trim().to_string();
                debug!("Found {} in project config: {}", key, value);
                return Some(value);
            }
        }
    }

    // Fallback to global config
    // Use --get-all to handle duplicate entries and take the last one
    if let Ok(output) = std::process::Command::new("git")
        .args(["config", "--global", "--get-all", key])
        .output()
    {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            // Get the last non-empty line (git uses last value when there are duplicates)
            if let Some(value) = output_str.lines().filter(|l| !l.trim().is_empty()).next_back() {
                let value = value.trim().to_string();
                debug!("Found {} in global config: {}", key, value);
                return Some(value);
            }
        }
    }

    // Fallback to system config
    // Use --get-all to handle duplicate entries and take the last one
    if let Ok(output) = std::process::Command::new("git")
        .args(["config", "--system", "--get-all", key])
        .output()
    {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            // Get the last non-empty line (git uses last value when there are duplicates)
            if let Some(value) = output_str.lines().filter(|l| !l.trim().is_empty()).next_back() {
                let value = value.trim().to_string();
                debug!("Found {} in system config: {}", key, value);
                return Some(value);
            }
        }
    }

    debug!("No value found for {} in any git config", key);
    None
}

/// Read all relevant git config values from the host
/// Returns a HashMap of config keys and their values
fn get_all_git_config_values(cwd: &std::path::Path) -> std::collections::HashMap<String, String> {
    use std::collections::HashMap;
    use tracing::debug;

    let mut config_map = HashMap::new();

    // List of git config keys we want to read
    let config_keys = vec![
        "user.name",
        "user.email",
        "user.signingkey",
        "commit.gpgsign",
        "gpg.format",
        "gpg.program",
        "core.editor",
        "init.defaultbranch",
        "pull.rebase",
        "push.autosetupremote",
    ];

    for key in config_keys {
        if let Some(value) = get_git_config(key, cwd) {
            debug!("Read git config: {} = {}", key, value);
            config_map.insert(key.to_string(), value);
        }
    }

    config_map
}

/// Generate .gitconfig file content from git config values
fn generate_gitconfig_content(config_map: &std::collections::HashMap<String, String>) -> String {
    use std::collections::HashMap;

    let mut content = String::from("# Generated by jail-ai from host git config\n\n");

    // Group config by section
    let mut sections: HashMap<String, Vec<(String, String)>> = HashMap::new();

    for (key, value) in config_map {
        if let Some((section, name)) = key.split_once('.') {
            sections
                .entry(section.to_string())
                .or_default()
                .push((name.to_string(), value.clone()));
        }
    }

    // Write sections in order
    let section_order = vec!["user", "commit", "gpg", "core", "init", "pull", "push"];

    for section_name in section_order {
        if let Some(entries) = sections.get(section_name) {
            content.push_str(&format!("[{}]\n", section_name));
            for (name, value) in entries {
                content.push_str(&format!("\t{} = {}\n", name, value));
            }
            content.push('\n');
        }
    }

    content
}

/// Create a .gitconfig file inside the container
/// Reads git config from host and creates the file directly inside the container
async fn create_gitconfig_in_container(cwd: &std::path::Path, jail: &jail::JailManager) -> error::Result<()> {
    use tracing::debug;

    // Read all relevant git config values from host
    let config_map = get_all_git_config_values(cwd);

    if config_map.is_empty() {
        debug!("No git config values found on host, skipping .gitconfig creation");
        return Ok(());
    }

    // Generate .gitconfig content
    let gitconfig_content = generate_gitconfig_content(&config_map);
    debug!("Generated .gitconfig content:\n{}", gitconfig_content);

    // Create the .gitconfig file inside the container using a shell command
    let create_file_cmd = vec![
        "sh".to_string(),
        "-c".to_string(),
        format!("cat > /home/agent/.gitconfig << 'GITCONFIG_EOF'\n{}\nGITCONFIG_EOF", gitconfig_content),
    ];

    jail.exec(&create_file_cmd, false).await?;

    info!("Created .gitconfig inside container with host's git configuration");

    Ok(())
}

/// Create a .claude.json file inside the container
/// Reads oauthAccount and userID from host's ~/.claude.json if it exists
/// Creates the file directly inside the container, not as a mount
async fn create_claude_json_in_container(home_path: &std::path::Path, jail: &jail::JailManager) -> error::Result<()> {
    use serde_json::{json, Value};

    let host_claude_json = home_path.join(".claude.json");

    let mut claude_json = json!({
        "hasCompletedOnboarding": true,
        "bypassPermissionsModeAccepted": true
    });

    // Try to read host's .claude.json and extract oauthAccount and userID
    if host_claude_json.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&host_claude_json).await {
            if let Ok(host_data) = serde_json::from_str::<Value>(&content) {
                // Copy oauthAccount if present
                if let Some(oauth_account) = host_data.get("oauthAccount") {
                    claude_json["oauthAccount"] = oauth_account.clone();
                }

                // Copy userID if present
                if let Some(user_id) = host_data.get("userID") {
                    claude_json["userID"] = user_id.clone();
                }
            }
        }
    }

    let json_content = serde_json::to_string(&claude_json)?;

    // Create the .claude.json file inside the container using a shell command
    // We use cat with a heredoc to create the file
    let create_file_cmd = vec![
        "sh".to_string(),
        "-c".to_string(),
        format!("mkdir -p /home/agent && cat > /home/agent/.claude.json << 'CLAUDE_JSON_EOF'\n{}\nCLAUDE_JSON_EOF", json_content),
    ];

    jail.exec(&create_file_cmd, false).await?;

    info!("Created .claude.json inside container");

    Ok(())
}

/// Get the jail-ai config directory path (XDG_CONFIG_HOME or ~/.config/jail-ai)
fn get_jail_ai_config_dir() -> error::Result<PathBuf> {
    let base_dir = if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(config_home)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config")
    } else {
        return Err(error::JailError::Config(
            "Could not determine config directory (HOME not set)".to_string(),
        ));
    };

    Ok(base_dir.join("jail-ai"))
}

/// Get the user's UID for runtime directory detection
fn get_user_uid() -> error::Result<u32> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let home = std::env::var("HOME").map_err(|_| {
            error::JailError::Config("HOME environment variable not set".to_string())
        })?;
        let metadata = std::fs::metadata(&home)?;
        Ok(metadata.uid())
    }

    #[cfg(not(unix))]
    {
        Err(error::JailError::Config(
            "UID detection not supported on non-Unix systems".to_string(),
        ))
    }
}

/// Prepare GPG configuration by resolving symlinks and copying to a persistent directory
/// This handles NixOS where config files are symlinks to /nix/store
/// Returns (persistent_dir_path, sockets_to_mount) where sockets need to be mounted separately
fn prepare_gpg_config(gpg_dir: &std::path::Path) -> error::Result<(std::path::PathBuf, Vec<std::path::PathBuf>)> {
    use std::os::unix::fs::FileTypeExt;
    use tracing::debug;

    if !gpg_dir.exists() {
        return Err(error::JailError::Config(format!("GPG directory does not exist: {}", gpg_dir.display())));
    }

    // Create a persistent directory for GPG config in ~/.config/jail-ai/gpg-cache
    let cache_dir = get_jail_ai_config_dir()?.join("gpg-cache");
    std::fs::create_dir_all(&cache_dir)?;

    // Create a unique directory based on the source GPG directory path
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(gpg_dir.to_string_lossy().as_bytes());
    let hash = hex::encode(hasher.finalize());
    let persistent_dir = cache_dir.join(&hash[..16]);

    // Remove old cache if it exists to ensure fresh config
    if persistent_dir.exists() {
        debug!("Removing old GPG cache: {}", persistent_dir.display());
        std::fs::remove_dir_all(&persistent_dir)?;
    }

    std::fs::create_dir_all(&persistent_dir)?;
    info!("Preparing GPG config in cache directory: {}", persistent_dir.display());
    debug!("Source GPG directory: {}", gpg_dir.display());

    let mut sockets = Vec::new();

    // Look for GPG agent sockets in the runtime directory (/run/user/UID/gnupg/)
    // This is where gpg-agent actually creates the sockets
    let uid = get_user_uid()?;
    let runtime_gpg_dir = std::path::PathBuf::from(format!("/run/user/{}/gnupg", uid));

    if runtime_gpg_dir.exists() {
        debug!("Checking runtime GPG directory: {}", runtime_gpg_dir.display());

        // Common GPG agent socket names
        let socket_names = vec![
            "S.gpg-agent",
            "S.gpg-agent.extra",
            "S.gpg-agent.ssh",
            "S.gpg-agent.browser",
        ];

        for socket_name in socket_names {
            let socket_path = runtime_gpg_dir.join(socket_name);
            if socket_path.exists() {
                let metadata = std::fs::symlink_metadata(&socket_path)?;
                if metadata.file_type().is_socket() {
                    debug!("Found runtime socket: {}", socket_path.display());
                    sockets.push(socket_path);
                }
            }
        }
    } else {
        debug!("Runtime GPG directory does not exist: {}", runtime_gpg_dir.display());
    }

    // Also check ~/.gnupg for any sockets (fallback)
    for entry in std::fs::read_dir(gpg_dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();

        // Skip directories (private-keys-v1.d, crls.d, public-keys.d)
        if path.is_dir() {
            debug!("Skipping directory: {}", file_name_str);
            continue;
        }

        let metadata = std::fs::symlink_metadata(&path)?;
        let file_type = metadata.file_type();

        // Socket files need to be mounted directly, not copied
        if file_type.is_socket() {
            debug!("Found socket in ~/.gnupg: {}", file_name_str);
            // Only add if not already in the list from runtime dir
            if !sockets.iter().any(|s| s.file_name() == Some(file_name.as_os_str())) {
                sockets.push(path.clone());
            }
            continue;
        }

        // Skip gpg-agent.conf from the host - we'll generate our own
        if file_name_str == "gpg-agent.conf" {
            debug!("Skipping host gpg-agent.conf - will generate custom config");
            continue;
        }

        // For regular files and symlinks, resolve and copy the content
        if file_type.is_file() || file_type.is_symlink() {
            let target_path = persistent_dir.join(&file_name);

            if file_type.is_symlink() {
                // Resolve symlink and copy the actual file content
                let symlink_target = std::fs::read_link(&path)?;
                info!("Resolving GPG config symlink: {} -> {}", file_name_str, symlink_target.display());
                let content = std::fs::read(&path)?;
                let content_len = content.len();
                std::fs::write(&target_path, content)?;
                debug!("Copied {} bytes from resolved symlink {} to cache", content_len, file_name_str);
            } else {
                // Copy regular file
                std::fs::copy(&path, &target_path)?;
                debug!("Copied regular file {} to cache", file_name_str);
            }

            // Preserve permissions
            #[cfg(unix)]
            {
                let perms = std::fs::metadata(&path)?.permissions();
                std::fs::set_permissions(&target_path, perms)?;
            }
        }
    }

    // Copy directories (private-keys-v1.d, crls.d, public-keys.d)
    for dir_name in &["private-keys-v1.d", "crls.d", "public-keys.d"] {
        let src_dir = gpg_dir.join(dir_name);
        if src_dir.exists() && src_dir.is_dir() {
            let dst_dir = persistent_dir.join(dir_name);
            std::fs::create_dir_all(&dst_dir)?;
            debug!("Copying directory: {}", dir_name);

            copy_dir_recursive(&src_dir, &dst_dir)?;
        }
    }

    // Generate custom gpg-agent.conf for the jail with pinentry-curses
    let gpg_agent_conf_path = persistent_dir.join("gpg-agent.conf");
    let gpg_agent_conf_content = "\
# Generated by jail-ai for container use
# Using pinentry-curses for terminal-based PIN entry
pinentry-program /usr/bin/pinentry-curses
enable-ssh-support
grab
allow-preset-passphrase
max-cache-ttl 86400
default-cache-ttl 3600
";
    std::fs::write(&gpg_agent_conf_path, gpg_agent_conf_content)?;
    info!("Generated custom gpg-agent.conf with pinentry-curses");
    debug!("gpg-agent.conf path: {}", gpg_agent_conf_path.display());

    // Set proper permissions for gpg-agent.conf (0600)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&gpg_agent_conf_path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&gpg_agent_conf_path, perms)?;
        debug!("Set gpg-agent.conf permissions to 0600");
    }

    info!("Prepared GPG config in persistent cache directory: {}", persistent_dir.display());
    debug!("Found {} GPG agent sockets to mount", sockets.len());
    Ok((persistent_dir, sockets))
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> error::Result<()> {
    use tracing::debug;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);

        let metadata = std::fs::symlink_metadata(&path)?;
        let file_type = metadata.file_type();

        if file_type.is_dir() {
            std::fs::create_dir_all(&dst_path)?;
            copy_dir_recursive(&path, &dst_path)?;
        } else if file_type.is_file() || file_type.is_symlink() {
            // Resolve symlinks and copy content
            let content = std::fs::read(&path)?;
            std::fs::write(&dst_path, content)?;

            // Preserve permissions
            #[cfg(unix)]
            {
                let perms = std::fs::metadata(&path)?.permissions();
                std::fs::set_permissions(&dst_path, perms)?;
            }

            debug!("Copied {} to {}", path.display(), dst_path.display());
        }
        // Skip sockets and other special files in subdirectories
    }

    Ok(())
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
    gemini_dir: bool,
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

        // Mount minimal auth files for Claude agent (unless full .claude directory is mounted)
        if agent_command == "claude" && !params.claude_dir && !params.agent_configs {
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

        // Opt-in: Mount ~/.config/gemini for Gemini CLI
        if params.gemini_dir || params.agent_configs {
            let gemini_config = home_path.join(".config").join("gemini");
            if gemini_config.exists() {
                info!(
                    "Mounting {} to /home/agent/.config/gemini",
                    gemini_config.display()
                );
                builder = builder.bind_mount(gemini_config, "/home/agent/.config/gemini", false);
            }
        }

        // Opt-in: GPG configuration
        if params.git_gpg {
            // Prepare and mount GPG configuration directory
            let gpg_dir = home_path.join(".gnupg");
            if gpg_dir.exists() {
                match prepare_gpg_config(&gpg_dir) {
                    Ok((temp_gpg_dir, sockets)) => {
                        info!(
                            "Mounting prepared GPG config {} to /home/agent/.gnupg",
                            temp_gpg_dir.display()
                        );
                        builder = builder.bind_mount(&temp_gpg_dir, "/home/agent/.gnupg", false);

                        // Mount GPG agent sockets for YubiKey and smartcard support
                        for socket_path in sockets {
                            if let Some(socket_name) = socket_path.file_name() {
                                let socket_name_str = socket_name.to_string_lossy();
                                let target = format!("/home/agent/.gnupg/{}", socket_name_str);
                                info!(
                                    "Mounting GPG socket {} to {}",
                                    socket_path.display(),
                                    target
                                );
                                builder = builder.bind_mount(socket_path, target, false);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to prepare GPG config: {}", e);
                    }
                }
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

        // Create .claude.json file inside the container for Claude agent
        if agent_command == "claude" {
            if let Err(e) = create_claude_json_in_container(&home_path, &jail).await {
                warn!("Failed to create .claude.json in container: {}", e);
            }
        }

        // Create .gitconfig file inside the container if git_gpg is enabled
        if params.git_gpg {
            if let Err(e) = create_gitconfig_in_container(&cwd, &jail).await {
                warn!("Failed to create .gitconfig in container: {}", e);
            }
        }
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

/// Upgrade a single jail with the specified image
async fn upgrade_single_jail(
    jail_name: &str,
    image: Option<String>,
    force: bool,
) -> error::Result<()> {
    // Create a temporary jail manager to inspect the existing configuration
    let temp_config = JailConfig {
        name: jail_name.to_string(),
        ..Default::default()
    };
    let temp_jail = jail::JailManager::new(temp_config);

    // Check if jail exists
    if !temp_jail.exists().await? {
        return Err(error::JailError::NotFound(format!(
            "Jail '{}' does not exist",
            jail_name
        )));
    }

    // Inspect the jail to get its current configuration
    let old_config = temp_jail.inspect().await?;
    info!("Current jail configuration: {:?}", old_config);

    // Determine the new image to use
    let new_image = image.unwrap_or_else(|| old_config.base_image.clone());

    // Ask for confirmation unless force is specified
    if !force {
        use std::io::{self, BufRead, Write};
        println!("Jail '{}' will be upgraded:", jail_name);
        println!("  Current image: {}", old_config.base_image);
        println!("  New image:     {}", new_image);
        println!("\nThis will:");
        println!("  1. Save the current configuration");
        println!("  2. Remove the existing jail");
        println!("  3. Recreate the jail with the new image");
        println!("  4. Restore the configuration (mounts, env, limits)");
        print!("\nProceed with upgrade? [y/N] ");
        io::stdout().flush()?;
        let stdin = io::stdin();
        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        if !line.trim().eq_ignore_ascii_case("y") {
            info!("Upgrade aborted");
            return Ok(());
        }
    }

    info!("Upgrading jail '{}'...", jail_name);

    // Remove the old jail (keep volume to preserve data)
    info!("Removing old jail...");
    temp_jail.remove(false).await?;
    info!("Old jail removed");

    // Create a new jail with the updated image but same configuration
    info!("Creating new jail with image '{}'...", new_image);
    let mut builder = JailBuilder::new(jail_name)
        .backend(old_config.backend)
        .base_image(new_image.clone())
        .network(old_config.network.enabled, old_config.network.private);

    // Restore environment variables
    for (key, value) in &old_config.environment {
        builder = builder.env(key.clone(), value.clone());
    }

    // Restore bind mounts
    for mount in &old_config.bind_mounts {
        builder = builder.bind_mount(mount.source.clone(), mount.target.clone(), mount.readonly);
    }

    // Restore resource limits
    if let Some(memory) = old_config.limits.memory_mb {
        builder = builder.memory_limit(memory);
    }
    if let Some(cpu) = old_config.limits.cpu_quota {
        builder = builder.cpu_quota(cpu);
    }

    let new_jail = builder.build();
    new_jail.create().await?;

    // Create .claude.json file inside the container if needed
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let home_path = std::path::PathBuf::from(&home);
    if let Err(e) = create_claude_json_in_container(&home_path, &new_jail).await {
        warn!("Failed to create .claude.json in container: {}", e);
    }

    println!(
        "✓ Jail '{}' successfully upgraded to image '{}'",
        jail_name, new_image
    );
    info!("Upgrade completed successfully");

    Ok(())
}

/// Upgrade all jails with the specified image (or their current image if not specified)
async fn upgrade_all_jails(image: Option<String>, force: bool) -> error::Result<()> {
    // Determine which backends to upgrade
    let backends = config::BackendType::all_available();
    if backends.is_empty() {
        warn!("No backends are available on this system");
        return Ok(());
    }

    let mut all_jails = Vec::new();

    // Collect all jails from all backends
    for backend_type in &backends {
        let temp_config = JailConfig {
            name: "temp".to_string(),
            backend: *backend_type,
            ..Default::default()
        };
        let backend = backend::create_backend(&temp_config);
        let jails = backend.list_all().await?;

        for jail_name in jails {
            all_jails.push((jail_name, *backend_type));
        }
    }

    if all_jails.is_empty() {
        println!("No jails found to upgrade");
        return Ok(());
    }

    info!("Found {} jail(s) to upgrade", all_jails.len());

    // Ask for confirmation unless force is specified
    if !force {
        use std::io::{self, BufRead, Write};
        println!("The following {} jail(s) will be upgraded:", all_jails.len());
        for (jail_name, backend_type) in &all_jails {
            println!("  - {} (backend: {:?})", jail_name, backend_type);
        }
        if let Some(ref img) = image {
            println!("\nAll jails will be upgraded to image: {}", img);
        } else {
            println!("\nEach jail will be upgraded to its current image (refreshed)");
        }
        print!("\nProceed with upgrade? [y/N] ");
        io::stdout().flush()?;
        let stdin = io::stdin();
        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        if !line.trim().eq_ignore_ascii_case("y") {
            info!("Upgrade aborted");
            return Ok(());
        }
    }

    // Upgrade each jail
    let mut success_count = 0;
    let mut error_count = 0;

    for (jail_name, _backend_type) in all_jails {
        info!("Upgrading jail: {}", jail_name);
        match upgrade_single_jail(&jail_name, image.clone(), true).await {
            Ok(_) => {
                success_count += 1;
            }
            Err(e) => {
                error!("Failed to upgrade jail {}: {}", jail_name, e);
                error_count += 1;
            }
        }
    }

    println!(
        "\n✓ Upgrade complete: {} succeeded, {} failed",
        success_count, error_count
    );
    info!("Upgrade-all operation completed");

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

    #[test]
    fn test_find_jails_for_directory_filters_correctly() {
        use std::path::PathBuf;

        // Test that we correctly filter jail names by base pattern
        let path = PathBuf::from("/tmp/test-project");
        let base_name = cli::Commands::generate_jail_name(&path);

        // Simulate jail names returned by backend
        let all_jails = vec![
            format!("{}-claude", base_name),
            format!("{}-copilot", base_name),
            format!("{}-cursor", base_name),
            "jail-other-12345678-claude".to_string(),
        ];

        // Filter logic from find_jails_for_directory
        let matching_jails: Vec<String> = all_jails
            .into_iter()
            .filter(|name| name.starts_with(&base_name) && name.len() > base_name.len())
            .collect();

        // Should match only the jails with the correct base pattern
        assert_eq!(matching_jails.len(), 3);
        assert!(matching_jails.contains(&format!("{}-claude", base_name)));
        assert!(matching_jails.contains(&format!("{}-copilot", base_name)));
        assert!(matching_jails.contains(&format!("{}-cursor", base_name)));
    }

    #[test]
    fn test_resolve_jail_name_logic() {
        use std::path::PathBuf;

        // Test that auto_detect_jail_name generates base name without agent suffix
        let path = PathBuf::from("/tmp/test-project");
        let base_name = cli::Commands::generate_jail_name(&path);

        // This should NOT include agent suffix
        assert!(!base_name.ends_with("-claude"));
        assert!(!base_name.ends_with("-copilot"));
        assert!(!base_name.ends_with("-cursor"));

        // The base name should be in format "jail-{dir}-{hash}"
        assert!(base_name.starts_with("jail-"));
        assert!(base_name.contains("test-project"));
    }

    #[tokio::test]
    async fn test_upgrade_single_jail_nonexistent() {
        // Test that upgrading a non-existent jail returns an error
        let result = upgrade_single_jail("nonexistent-jail", None, true).await;
        assert!(result.is_err());
        if let Err(e) = result {
            match e {
                error::JailError::NotFound(_) => {
                    // Expected error type
                }
                _ => panic!("Expected NotFound error, got {:?}", e),
            }
        }
    }

    #[tokio::test]
    async fn test_upgrade_all_jails_empty() {
        // Test that upgrading with no jails doesn't fail
        // This will succeed but do nothing if no jails exist
        let result = upgrade_all_jails(None, true).await;
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(unix)]
    fn test_get_user_uid() {
        // Test that we can get the user's UID
        let uid = get_user_uid();
        assert!(uid.is_ok());
        let uid_val = uid.unwrap();
        assert!(uid_val > 0, "UID should be greater than 0");
    }

    #[test]
    fn test_get_git_config() {
        // Test that we can read git config values
        // This test will try to read from local/global/system config
        let cwd = std::env::current_dir().unwrap();

        // Try to get any git config value - user.name is commonly set
        // If no git config exists, this test will pass (returns None)
        let result = get_git_config("user.name", &cwd);

        // The test passes whether or not git config exists
        // We're just testing that the function doesn't panic
        if let Some(name) = result {
            // If we found a name, it should not be empty
            assert!(!name.is_empty(), "Git config value should not be empty");
        }
    }

    #[test]
    fn test_get_git_config_hierarchy() {
        // Test that get_git_config respects config hierarchy
        // local > global > system
        let cwd = std::env::current_dir().unwrap();

        // Test with a non-existent key
        let result = get_git_config("nonexistent.key.that.should.not.exist", &cwd);
        assert!(result.is_none(), "Non-existent git config key should return None");
    }
}
