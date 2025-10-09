use crate::cli::Commands;
use crate::config::{BackendType, JailConfig};
use crate::error::{self, Result};
use crate::git_gpg::{
    create_claude_json_in_container, create_gitconfig_in_container, setup_git_gpg_config,
};
use crate::jail::{JailBuilder, JailManager};
use crate::jail_setup::{self, mount_agent_configs, setup_default_environment};
use std::path::{Path, PathBuf};
use tracing::info;

/// Parameters for AI agent commands
pub struct AgentCommandParams {
    pub backend: Option<String>,
    pub image: String,
    pub mount: Vec<String>,
    pub env: Vec<String>,
    pub no_network: bool,
    pub memory: Option<u64>,
    pub cpu: Option<u32>,
    pub no_workspace: bool,
    pub workspace_path: String,
    pub claude_dir: bool,
    pub copilot_dir: bool,
    pub cursor_dir: bool,
    pub gemini_dir: bool,
    pub codex_dir: bool,
    pub agent_configs: bool,
    pub git_gpg: bool,
    pub force_rebuild: bool,
    pub force_layers: Vec<String>,
    pub shell: bool,
    pub isolated: bool,
    pub verbose: bool,
    pub auth: Option<String>,
    pub args: Vec<String>,
}

/// Auto-detect jail name from current directory (or git root if available)
pub fn auto_detect_jail_name() -> Result<String> {
    let cwd = std::env::current_dir()?;
    let workspace_dir = get_git_root().unwrap_or(cwd);
    let jail_name = Commands::generate_jail_name(&workspace_dir);
    info!("Auto-detected jail name from workspace: {}", jail_name);
    Ok(jail_name)
}

/// Get the git root directory if the current directory is within a git repository
pub fn get_git_root() -> Option<PathBuf> {
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

/// Map agent command names to normalized agent identifiers
/// (e.g., "cursor-agent" -> "cursor")
fn normalize_agent_name(agent_command: &str) -> &str {
    match agent_command {
        "cursor-agent" => "cursor",
        _ => agent_command,
    }
}

/// Helper function to run AI agent commands (claude, copilot, cursor-agent, gemini)
pub async fn run_ai_agent_command(agent_command: &str, params: AgentCommandParams) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let base_name = auto_detect_jail_name()?;

    // Normalize agent name for jail naming and image building
    let normalized_agent = normalize_agent_name(agent_command);
    let agent_suffix = Commands::sanitize_jail_name(normalized_agent);
    let jail_name = format!("{base_name}-{agent_suffix}");

    info!("Using jail: {} for agent: {}", jail_name, agent_command);

    // Determine backend type early - use user-specified or auto-detect
    let backend_type = if let Some(backend_str) = params.backend {
        Commands::parse_backend(&backend_str).map_err(error::JailError::Config)?
    } else {
        BackendType::detect()
    };

    // Check if we need to create/recreate the jail
    // Force recreation if --force-rebuild or --layers is specified
    let temp_config = JailConfig {
        name: jail_name.clone(),
        backend: backend_type,
        ..Default::default()
    };
    let temp_jail = JailManager::new(temp_config);
    let jail_exists = temp_jail.exists().await?;
    let should_recreate = params.force_rebuild || !params.force_layers.is_empty();

    if !jail_exists {
        info!("Creating new jail: {}", jail_name);
    } else if should_recreate {
        info!("Jail exists but recreation requested (force_rebuild={}, force_layers={:?})", 
              params.force_rebuild, params.force_layers);
    }

    if !jail_exists || should_recreate {
        if jail_exists && should_recreate {
            info!("Recreating jail '{}' due to --force-rebuild or --layers", jail_name);
        }

        // Only use custom image if explicitly provided (not default)
        // This allows the layered image system to auto-detect and build agent-specific images
        let use_custom_image = params.image != crate::cli::DEFAULT_IMAGE;

        let mut builder = JailBuilder::new(&jail_name)
            .backend(backend_type)
            .network(!params.no_network, true);

        // Set image: use custom if provided, otherwise let layered system auto-detect
        if use_custom_image {
            builder = builder.base_image(params.image);
        } else {
            // Use default image name, which triggers layered image auto-detection
            builder = builder.base_image(crate::image::DEFAULT_IMAGE_NAME);
        }

        // Setup default environment variables
        builder = setup_default_environment(builder);

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

        // Auto-mount agent config directories
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let home_path = PathBuf::from(&home);

        builder = mount_agent_configs(
            builder,
            &home_path,
            agent_command,
            &jail_setup::AgentConfigFlags {
                claude_dir: params.claude_dir,
                copilot_dir: params.copilot_dir,
                cursor_dir: params.cursor_dir,
                gemini_dir: params.gemini_dir,
                codex_dir: params.codex_dir,
                agent_configs: params.agent_configs,
            },
        );

        // Opt-in: GPG configuration
        if params.git_gpg {
            builder = setup_git_gpg_config(builder, &cwd, &home_path)?;
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

        // Set force rebuild flag
        builder = builder.force_rebuild(params.force_rebuild);

        // Set force layers
        builder = builder.force_layers(params.force_layers);

        // Set isolated flag
        builder = builder.isolated(params.isolated);

        // Set verbose flag
        builder = builder.verbose(params.verbose);

        let jail = builder.build();
        jail.create().await?;

        // Create .claude.json file inside the container for Claude agent
        if agent_command == "claude" {
            if let Err(e) = create_claude_json_in_container(&home_path, &jail).await {
                tracing::warn!("Failed to create .claude.json in container: {}", e);
            }
        }

        // Create .gitconfig file inside the container if git_gpg is enabled
        if params.git_gpg {
            if let Err(e) = create_gitconfig_in_container(&cwd, &jail).await {
                tracing::warn!("Failed to create .gitconfig in container: {}", e);
            }
        }
    }

    // Execute AI agent command (use the same backend type determined earlier)
    let jail = JailBuilder::new(&jail_name)
        .backend(backend_type)
        .verbose(params.verbose)
        .build();

    // If --shell flag is set, start an interactive shell instead of running the agent
    if params.shell {
        info!("Starting interactive shell in jail '{}'...", jail_name);
        let output = jail.exec(&["/usr/bin/zsh".to_string()], true).await?;
        if !output.is_empty() {
            print!("{output}");
        }
        return Ok(());
    }

    // Handle Codex CLI authentication if --auth key is provided
    if agent_command == "codex --dangerously-bypass-approvals-and-sandbox" {
        if let Some(api_key) = &params.auth {
            info!("Codex CLI authentication with provided API key...");
            // Use echo to pipe the API key to codex login --with-api-key
            let auth_cmd = vec![
                "sh".to_string(),
                "-c".to_string(),
                format!("echo '{}' | codex login --with-api-key", api_key),
            ];
            let auth_output = jail.exec(&auth_cmd, false).await?;
            if !auth_output.is_empty() {
                print!("{auth_output}");
            }
        }
    }

    let mut command = vec![agent_command.to_string()];
    command.extend(params.args);

    let output = jail.exec(&command, true).await?;
    if !output.is_empty() {
        print!("{output}");
    }

    Ok(())
}

/// Find all jails matching the current directory pattern
pub async fn find_jails_for_directory(workspace_dir: &Path) -> Result<Vec<String>> {
    let base_name = Commands::generate_jail_name(workspace_dir);
    let backend_type = BackendType::detect();

    // Create a temporary config just to access the backend
    let temp_config = JailConfig {
        name: "temp".to_string(),
        backend: backend_type,
        ..Default::default()
    };
    let backend = crate::backend::create_backend(&temp_config);

    // List all jails
    let all_jails = backend.list_all().await?;

    // Filter jails that match the base pattern (jail-{project}-{hash}-)
    let matching_jails: Vec<String> = all_jails
        .into_iter()
        .filter(|name| name.starts_with(&base_name) && name.len() > base_name.len())
        .collect();

    Ok(matching_jails)
}

/// Extract agent name from jail name for display purposes
/// Jail name format: jail-{project}-{hash}-{agent}
/// Returns a simplified agent name for display (e.g., "cursor" instead of "cursor-agent")
pub fn extract_agent_name(jail_name: &str) -> &str {
    // The format is: jail-{project}-{hash}-{agent}
    // The hash is always 8 characters, so we need to find it and take everything after it

    if jail_name.starts_with("jail-") {
        // Find the hash part (8 characters) and take everything after it
        let parts: Vec<&str> = jail_name.split('-').collect();

        // Look for a part that is exactly 8 characters (the hash)
        for (i, part) in parts.iter().enumerate() {
            if part.len() == 8 && part.chars().all(|c| c.is_ascii_hexdigit()) {
                // Found the hash at index i, agent starts after the next dash
                if i + 1 < parts.len() {
                    // Find the position of the agent part in the original string
                    // Count characters up to and including the hash, then skip the next dash
                    let mut pos = 0;
                    for (j, p) in parts.iter().enumerate() {
                        if j <= i {
                            pos += p.len() + 1; // +1 for the dash
                        } else {
                            break;
                        }
                    }
                    // Skip the dash after the hash
                    if pos < jail_name.len() && jail_name.chars().nth(pos) == Some('-') {
                        pos += 1;
                    }
                    let agent_part = &jail_name[pos..];

                    // Simplify common agent names for display
                    match agent_part {
                        "cursor-agent" => return "cursor",
                        "claude" => return "claude",
                        "copilot" => return "copilot",
                        "gemini" => return "gemini",
                        "codex" => return "codex",
                        _ => return agent_part, // Return as-is for other agents
                    }
                }
            }
        }
    }

    // Fallback: just take the last part
    jail_name.split('-').next_back().unwrap_or("unknown")
}

/// Prompt user to select a jail from a list
pub fn select_jail(jails: &[String]) -> Result<String> {
    use std::io::{self, Write};

    println!("Multiple jails found for this directory:");
    for (i, jail) in jails.iter().enumerate() {
        // Extract agent name from jail name
        let agent_name = extract_agent_name(jail);
        println!("  {}. {} (agent: {})", i + 1, jail, agent_name);
    }

    print!("Select a jail (1-{}): ", jails.len());
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| error::JailError::Config(format!("Failed to read input: {e}")))?;

    let selection: usize = input
        .trim()
        .parse()
        .map_err(|_| error::JailError::Config("Invalid selection".to_string()))?;

    if selection < 1 || selection > jails.len() {
        return Err(error::JailError::Config(
            "Selection out of range".to_string(),
        ));
    }

    Ok(jails[selection - 1].clone())
}
