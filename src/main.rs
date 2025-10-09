mod agent_commands;
mod backend;
mod cli;
mod config;
mod error;
mod git_gpg;
mod image;
mod image_layers;
mod jail;
mod jail_setup;
mod project_detection;

use clap::Parser;
use cli::{Cli, Commands};
use config::JailConfig;
use jail::JailBuilder;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Import functions from modules
use agent_commands::get_git_root;
use git_gpg::create_claude_json_in_container;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = if cli.verbose {
        "jail_ai=debug,info"
    } else {
        "jail_ai=warn"
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| filter.into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    if let Err(e) = run(cli.command, cli.verbose).await {
        error!("Error: {}", e);
        std::process::exit(1);
    }
}

async fn run(command: Option<Commands>, verbose: bool) -> error::Result<()> {
    match command {
        None => {
            // Default behavior: auto-init and exec based on workspace (git root if available)
            let cwd = std::env::current_dir()?;
            let workspace_dir = agent_commands::get_git_root().unwrap_or(cwd.clone());

            // Find all jails for this directory
            let matching_jails = agent_commands::find_jails_for_directory(&workspace_dir).await?;

            let jail_name = if matching_jails.is_empty() {
                // No jails exist, create a default one
                let base_name = cli::Commands::generate_jail_name(&workspace_dir);
                info!("No jail found for this directory, creating default jail...");
                let jail_name = format!("{base_name}-default");

                info!("Creating jail '{}'...", jail_name);
                let jail = create_default_jail(&jail_name, &workspace_dir, verbose).await?;
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
                agent_commands::select_jail(&matching_jails)?
            };

            // Exec into jail with interactive shell
            info!("Executing interactive shell in jail '{}'...", jail_name);
            let jail = JailBuilder::new(jail_name.clone())
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
                codex_dir,
                agent_configs,
                git_gpg,
                force_rebuild,
                layers,
                isolated,
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

                    // Setup default environment variables
                    builder = jail_setup::setup_default_environment(builder);

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

                    // Mount agent config directories
                    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
                    let home_path = std::path::PathBuf::from(&home);

                    builder = jail_setup::mount_agent_configs(
                        builder,
                        &home_path,
                        "", // No specific agent for create command
                        &jail_setup::AgentConfigFlags {
                            claude_dir,
                            copilot_dir,
                            cursor_dir,
                            gemini_dir,
                            codex_dir,
                            agent_configs,
                        },
                    );

                    // Opt-in: GPG configuration
                    if git_gpg {
                        let cwd = std::env::current_dir()?;
                        builder = git_gpg::setup_git_gpg_config(builder, &cwd, &home_path)?;
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

                    // Set force rebuild flag
                    builder = builder.force_rebuild(force_rebuild);

                    // Set force layers
                    builder = builder.force_layers(layers);

                    // Set isolated flag
                    builder = builder.isolated(isolated);

                    // Set verbose flag
                    builder = builder.verbose(verbose);

                    builder.build()
                };

                jail.create().await?;

                // Create .claude.json file inside the container
                let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
                let home_path = std::path::PathBuf::from(&home);
                if let Err(e) = git_gpg::create_claude_json_in_container(&home_path, &jail).await {
                    warn!("Failed to create .claude.json in container: {}", e);
                }

                // Create .gitconfig file inside the container if git_gpg is enabled
                if git_gpg {
                    let cwd = std::env::current_dir()?;
                    if let Err(e) = git_gpg::create_gitconfig_in_container(&cwd, &jail).await {
                        warn!("Failed to create .gitconfig in container: {}", e);
                    }
                }

                info!("Jail created: {}", jail.config().name);
            }

            Commands::Remove {
                name,
                force,
                volume,
            } => {
                let jail_name = resolve_jail_name(name).await?;

                if !force {
                    use std::io::{self, BufRead, Write};
                    print!("Remove jail '{jail_name}'? [y/N] ");
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
                        "Jail '{jail_name}' does not exist"
                    )));
                }

                // Inspect the jail to get its actual configuration
                let config = jail.inspect().await?;

                let json = serde_json::to_string_pretty(&config)?;
                tokio::fs::write(&output, json).await?;
                println!(
                    "✓ Configuration for jail '{}' saved to: {}",
                    jail_name,
                    output.display()
                );
                info!("Configuration saved to: {}", output.display());
            }

            Commands::Claude { common, args } => {
                run_agent_command("claude", common, args, verbose).await?;
            }

            Commands::Copilot { common, args } => {
                run_agent_command("copilot", common, args, verbose).await?;
            }

            Commands::Cursor { common, args } => {
                run_agent_command("cursor-agent", common, args, verbose).await?;
            }

            Commands::Gemini { common, args } => {
                run_agent_command("gemini", common, args, verbose).await?;
            }

            Commands::Codex { common, args } => {
                run_agent_command("codex", common, args, verbose).await?;
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
                    let workspace_dir = agent_commands::get_git_root().unwrap_or(cwd);
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
                        // Extract agent name from jail name
                        let agent_suffix = agent_commands::extract_agent_name(jail_name);

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

                        println!("  {jail_name} [{status}] ({agent_suffix})");
                    }
                    println!("\nTotal: {} jail(s)", jails.len());
                }
            }

            Commands::CleanAll {
                backend,
                force,
                volume,
            } => {
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
                            println!("  - {jail_name}");
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

            Commands::Upgrade {
                name,
                image,
                force,
                all,
            } => {
                if all {
                    // Upgrade all jails
                    upgrade_all_jails(image, force, verbose).await?;
                } else {
                    // Upgrade single jail
                    let jail_name = resolve_jail_name(name).await?;
                    upgrade_single_jail(&jail_name, image, force, verbose).await?;
                }
            }
        },
    }

    Ok(())
}

/// Helper function to run an AI agent command
/// This consolidates the common logic for all agent commands
async fn run_agent_command(
    agent_name: &str,
    common: cli::AgentCommandOptions,
    args: Vec<String>,
    verbose: bool,
) -> error::Result<()> {
    agent_commands::run_ai_agent_command(
        agent_name,
        agent_commands::AgentCommandParams {
            backend: common.backend,
            image: common.image,
            mount: common.mount,
            env: common.env,
            no_network: common.no_network,
            memory: common.memory,
            cpu: common.cpu,
            no_workspace: common.no_workspace,
            workspace_path: common.workspace_path,
            claude_dir: common.claude_dir,
            copilot_dir: common.copilot_dir,
            cursor_dir: common.cursor_dir,
            gemini_dir: common.gemini_dir,
            codex_dir: common.codex_dir,
            agent_configs: common.agent_configs,
            git_gpg: common.git_gpg,
            force_rebuild: common.force_rebuild,
            force_layers: common.layers,
            shell: common.shell,
            isolated: common.isolated,
            verbose,
            args,
        },
    )
    .await
}

/// Helper function to create a jail with default configuration
async fn create_default_jail(
    name: &str,
    workspace: &std::path::Path,
    verbose: bool,
) -> error::Result<jail::JailManager> {
    let backend_type = config::BackendType::detect();

    let mut builder = JailBuilder::new(name)
        .backend(backend_type)
        .base_image(image::DEFAULT_IMAGE_NAME)
        .network(true, true)
        .verbose(verbose);

    // Setup default environment variables
    builder = jail_setup::setup_default_environment(builder);

    // Auto-mount workspace (git root if available, otherwise provided workspace)
    let workspace_dir = get_git_root().unwrap_or(workspace.to_path_buf());
    info!("Auto-mounting {} to /workspace", workspace_dir.display());
    builder = builder.bind_mount(workspace_dir, "/workspace", false);

    Ok(builder.build())
}

/// Resolve jail name: use provided name or auto-detect from current directory
async fn resolve_jail_name(name: Option<String>) -> error::Result<String> {
    if let Some(name) = name {
        Ok(name)
    } else {
        // Auto-detect: find all matching jails for this directory
        let cwd = std::env::current_dir()?;
        let workspace_dir = agent_commands::get_git_root().unwrap_or(cwd);
        let matching_jails = agent_commands::find_jails_for_directory(&workspace_dir).await?;

        let jail_name = if matching_jails.is_empty() {
            return Err(error::JailError::Config(
                "No jails found for this directory. Create one first.".to_string(),
            ));
        } else if matching_jails.len() == 1 {
            // Only one jail exists, use it
            matching_jails[0].clone()
        } else {
            // Multiple jails exist, ask user to choose
            agent_commands::select_jail(&matching_jails)?
        };

        info!("Auto-detected jail: {}", jail_name);
        Ok(jail_name)
    }
}

async fn upgrade_single_jail(
    jail_name: &str,
    image: Option<String>,
    force: bool,
    verbose: bool,
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
            "Jail '{jail_name}' does not exist"
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
        println!("Jail '{jail_name}' will be upgraded:");
        println!("  Current image: {}", old_config.base_image);
        println!("  New image:     {new_image}");
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
        .network(old_config.network.enabled, old_config.network.private)
        .verbose(verbose);

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

    println!("✓ Jail '{jail_name}' successfully upgraded to image '{new_image}'");
    info!("Upgrade completed successfully");

    Ok(())
}

/// Upgrade all jails with the specified image (or their current image if not specified)
async fn upgrade_all_jails(image: Option<String>, force: bool, verbose: bool) -> error::Result<()> {
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
        println!(
            "The following {} jail(s) will be upgraded:",
            all_jails.len()
        );
        for (jail_name, backend_type) in &all_jails {
            println!("  - {jail_name} (backend: {backend_type:?})");
        }
        if let Some(ref img) = image {
            println!("\nAll jails will be upgraded to image: {img}");
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
        match upgrade_single_jail(&jail_name, image.clone(), true, verbose).await {
            Ok(_) => {
                success_count += 1;
            }
            Err(e) => {
                error!("Failed to upgrade jail {}: {}", jail_name, e);
                error_count += 1;
            }
        }
    }

    println!("\n✓ Upgrade complete: {success_count} succeeded, {error_count} failed");
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
        let result = upgrade_single_jail("nonexistent-jail", None, true, false).await;
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
        let result = upgrade_all_jails(None, true, false).await;
        assert!(result.is_ok());
    }
}
