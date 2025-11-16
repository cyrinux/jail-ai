use crate::cli::Commands;
use crate::config::{BackendType, JailConfig};
use crate::error::{self, Result};
use crate::git_gpg::{
    create_claude_json_in_container, create_gitconfig_in_container, setup_git_gpg_config,
};
use crate::jail::{JailBuilder, JailManager};
use crate::jail_setup::{self, mount_agent_configs, setup_default_environment};
use crate::strings;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Parameters for AI agent commands
pub struct AgentCommandParams {
    pub backend: Option<String>,
    pub image: String,
    pub mount: Vec<String>,
    pub port: Vec<String>,
    pub env: Vec<String>,
    pub no_network: bool,
    pub memory: Option<u64>,
    pub cpu: Option<u32>,
    pub no_workspace: bool,
    pub workspace_path: String,
    pub claude_dir: bool,
    pub claude_code_router_dir: bool,
    pub copilot_dir: bool,
    pub cursor_dir: bool,
    pub gemini_dir: bool,
    pub codex_dir: bool,
    pub jules_dir: bool,
    pub agent_configs: bool,
    pub git_gpg: bool,
    pub upgrade: bool,
    pub force_layers: Vec<String>,
    pub shell: bool,
    pub isolated: bool,
    pub verbose: bool,
    pub auth: bool,
    pub no_nix: bool,
    pub no_block_host: bool,
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

/// Validate that a workspace directory is safe for jail execution
/// Prevents execution in home directory root and system directories
pub fn validate_workspace_directory(workspace_dir: &Path) -> Result<()> {
    let workspace_dir = workspace_dir.canonicalize().map_err(error::JailError::Io)?;

    // Get the user's home directory
    let home_dir = std::env::var("HOME")
        .map_err(|_| error::JailError::Config("HOME environment variable not set".to_string()))?;
    let home_path = PathBuf::from(&home_dir)
        .canonicalize()
        .map_err(error::JailError::Io)?;

    // Check if workspace is the home directory root
    if workspace_dir == home_path {
        return Err(error::JailError::UnsafeWorkspace(format!(
            "Cannot run jail-ai in home directory root: {}",
            workspace_dir.display()
        )));
    }

    // Check if workspace is a system directory
    let system_dirs = [
        "/",
        "/bin",
        "/sbin",
        "/usr",
        "/usr/bin",
        "/usr/sbin",
        "/usr/local",
        "/etc",
        "/var",
        "/lib",
        "/lib64",
        "/opt",
        "/root",
        "/sys",
        "/proc",
        "/dev",
    ];

    for system_dir in &system_dirs {
        if let Ok(system_path) = PathBuf::from(system_dir).canonicalize() {
            if workspace_dir == system_path {
                return Err(error::JailError::UnsafeWorkspace(format!(
                    "Cannot run jail-ai in system directory: {}",
                    workspace_dir.display()
                )));
            }
        }
    }

    // Check if workspace is inside a system directory (but not root)
    for system_dir in &system_dirs {
        if *system_dir == "/" {
            // Skip root directory check as everything is under root
            continue;
        }

        if let Ok(system_path) = PathBuf::from(system_dir).canonicalize() {
            if workspace_dir.starts_with(&system_path) && workspace_dir != system_path {
                return Err(error::JailError::UnsafeWorkspace(format!(
                    "Cannot run jail-ai in system subdirectory: {}",
                    workspace_dir.display()
                )));
            }
        }
    }

    Ok(())
}

/// Map agent command names to normalized agent identifiers
/// (e.g., "cursor-agent" -> "cursor")
fn normalize_agent_name(agent_command: &str) -> &str {
    crate::agents::Agent::from_str(agent_command)
        .map(|a| a.normalized_name())
        .unwrap_or(agent_command)
}

/// Check if a container's image is outdated and needs an upgrade
/// Returns (needs_upgrade, current_image, expected_image)
async fn check_container_upgrade_needed(
    jail_name: &str,
    workspace_path: &Path,
    agent_name: &str,
    isolated: bool,
    no_nix: bool,
) -> Result<(bool, String, String)> {
    // Get the current image used by the container
    let backend = crate::backend::podman::PodmanBackend::new();
    let current_image = backend.get_container_image(jail_name).await?;

    // Determine what image should be used now based on current project state
    let expected_image = crate::image_layers::get_expected_image_name(
        workspace_path,
        Some(agent_name),
        isolated,
        no_nix,
    )
    .await?;

    // Check if images differ
    let needs_upgrade = current_image != expected_image;

    Ok((needs_upgrade, current_image, expected_image))
}

/// Prompt user to upgrade (for outdated layers or container)
fn prompt_upgrade() -> Result<bool> {
    use std::io::{self, Write};

    print!("{}", strings::WOULD_YOU_LIKE_REBUILD);
    io::stdout()
        .flush()
        .map_err(|e| error::JailError::Config(format!("Failed to flush stdout: {e}")))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| error::JailError::Config(format!("Failed to read input: {e}")))?;

    let answer = input.trim().to_lowercase();
    Ok(answer == "y" || answer == "yes")
}

/// Helper function to run AI agent commands (claude, copilot, cursor-agent, gemini)
pub async fn run_ai_agent_command(
    agent_command: &str,
    mut params: AgentCommandParams,
) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let base_name = auto_detect_jail_name()?;

    // Normalize agent name for jail naming and image building
    let normalized_agent = normalize_agent_name(agent_command);
    let agent_suffix = Commands::sanitize_jail_name(normalized_agent);
    let jail_name = format!("{base_name}__{agent_suffix}");

    info!("Using jail: {} for agent: {}", jail_name, agent_command);

    // Determine backend type early - use user-specified or auto-detect
    let backend_type = if let Some(backend_str) = params.backend {
        Commands::parse_backend(&backend_str).map_err(error::JailError::Config)?
    } else {
        BackendType::detect()
    };

    // Check if we need to create/recreate the jail
    // --upgrade and --layers force layer rebuilding + container recreation
    // --auth only forces container recreation (for host networking) without rebuilding layers

    // Auto-detect if agent needs authentication (first run - credentials missing or empty)
    // Only check if the user has mounted the config directory, otherwise credentials won't be accessible
    if !params.auth {
        if let Some(agent) = crate::agents::Agent::from_str(agent_command) {
            // Only auto-enable auth for agents that support the auth workflow
            if agent.supports_auth_workflow() {
                // Check if the appropriate config directory flag is set
                let config_dir_mounted = match agent {
                    crate::agents::Agent::Codex => params.codex_dir || params.agent_configs,
                    crate::agents::Agent::Jules => params.jules_dir || params.agent_configs,
                    _ => false, // Other agents don't use this auto-detection
                };

                // Only check for credentials if the config directory will be mounted
                if config_dir_mounted {
                    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
                    let home_path = PathBuf::from(&home);

                    if agent.needs_auth(&home_path) {
                        info!(
                            "First run detected - {} credentials not found or empty",
                            agent.display_name()
                        );
                        info!("Automatically enabling authentication mode");
                        params.auth = true;
                    }
                }
            }
        }
    }

    let temp_config = JailConfig {
        name: jail_name.clone(),
        backend: backend_type,
        ..Default::default()
    };
    let temp_jail = JailManager::new(temp_config);
    let jail_exists = temp_jail.exists().await?;

    // Track container recreation separately from layer rebuilding
    // --auth should force container recreation (for networking) but not layer rebuilding
    let mut should_recreate = params.upgrade || !params.force_layers.is_empty() || params.auth;

    // Check for network mode mismatch when container exists
    if jail_exists && !should_recreate {
        // Inspect existing container to get its network configuration
        if let Ok(existing_config) = temp_jail.inspect().await {
            // Determine desired network mode based on --auth flag
            let desired_host_network = params.auth;
            let current_host_network = existing_config.network.host;

            // If network mode has changed, force recreation
            if desired_host_network != current_host_network {
                if desired_host_network {
                    info!(
                        "Network mode mismatch detected: container has private networking but --auth requires host networking"
                    );
                } else {
                    info!(
                        "Network mode mismatch detected: container has host networking but normal mode requires private networking"
                    );
                    info!("Container will be recreated to restore secure network isolation");
                }
                should_recreate = true;
                // Note: We don't set params.upgrade = true here because we only need to
                // recreate the container, not rebuild the image layers
            }

            // Check for block_host mismatch
            let desired_block_host = !params.no_block_host;
            let current_block_host = existing_config.block_host;

            if desired_block_host != current_block_host {
                if desired_block_host {
                    info!(
                        "eBPF blocking mode mismatch detected: container has eBPF blocking disabled but current settings require it enabled"
                    );
                } else {
                    info!(
                        "eBPF blocking mode mismatch detected: container has eBPF blocking enabled but --no-block-host flag requires it disabled"
                    );
                }
                info!("Container will be recreated with the correct eBPF blocking configuration");
                should_recreate = true;
            }
        }
    }

    if !jail_exists {
        info!(
            "{}",
            strings::format_string(strings::CREATING_NEW_JAIL, &jail_name)
        );

        // Check if layers need rebuilding even for new jails
        // (e.g., after upgrading jail-ai binary, old cached layers may be outdated)
        let workspace_dir = get_git_root().unwrap_or_else(|| cwd.clone());
        let outdated_layers = match crate::image_layers::check_layers_need_rebuild(
            &workspace_dir,
            Some(normalized_agent),
            params.no_nix,
        )
        .await
        {
            Ok(layers) => layers,
            Err(e) => {
                warn!("Failed to check for outdated layers: {}", e);
                Vec::new()
            }
        };

        if !outdated_layers.is_empty() {
            info!("Outdated layers detected during jail creation:");
            for layer in &outdated_layers {
                info!("  • {}", layer);
            }
            info!("Layers will be rebuilt automatically");
            // Enable upgrade to ensure layers are rebuilt
            params.upgrade = true;
        }
    } else if should_recreate {
        info!(
            "Jail exists but recreation requested (upgrade={}, layers={:?})",
            params.upgrade, params.force_layers
        );
    } else {
        // Jail exists and no forced recreation - check if upgrade is available
        info!("{}", strings::CHECKING_UPDATES);

        let workspace_dir = get_git_root().unwrap_or_else(|| cwd.clone());

        // Check if any layers need rebuilding (e.g., after jail-ai binary upgrade)
        let outdated_layers = match crate::image_layers::check_layers_need_rebuild(
            &workspace_dir,
            Some(normalized_agent),
            params.no_nix,
        )
        .await
        {
            Ok(layers) => layers,
            Err(e) => {
                warn!("Failed to check for outdated layers: {}", e);
                Vec::new()
            }
        };

        // Check if container image is outdated
        let container_outdated = match check_container_upgrade_needed(
            &jail_name,
            &workspace_dir,
            normalized_agent,
            params.isolated,
            params.no_nix,
        )
        .await
        {
            Ok((needs_upgrade, current_img, expected_img)) => {
                if needs_upgrade {
                    Some((current_img, expected_img))
                } else {
                    None
                }
            }
            Err(e) => {
                warn!("Failed to check for container upgrade: {}", e);
                None
            }
        };

        // Determine what needs upgrading and prompt accordingly
        if !outdated_layers.is_empty() || container_outdated.is_some() {
            println!("{}", strings::UPDATE_AVAILABLE);

            if !outdated_layers.is_empty() {
                println!("{}", strings::OUTDATED_LAYERS_DETECTED);
                for layer in &outdated_layers {
                    println!("  • {}", layer);
                }
                println!("{}", strings::OUTDATED_LAYERS_EXPLAIN);
            }

            if let Some((ref current_img, ref expected_img)) = container_outdated {
                println!("{}", strings::CONTAINER_IMAGE_MISMATCH);
                println!("{}", strings::format_string(strings::CURRENT, current_img));
                println!(
                    "{}",
                    strings::format_string(strings::EXPECTED, expected_img)
                );
            }

            println!("{}", strings::RECOMMENDATION_USE_UPGRADE);
            if !outdated_layers.is_empty() {
                println!("{}", strings::REBUILD_OUTDATED_LAYERS);
            }
            if container_outdated.is_some() {
                println!("{}", strings::RECREATE_CONTAINER);
            }
            println!("{}", strings::ENSURE_LATEST_TOOLS);
            println!("{}", strings::DATA_PRESERVED);

            if prompt_upgrade()? {
                info!("{}", strings::USER_CHOSE_UPGRADE);
                should_recreate = true;
                // Enable upgrade to ensure layers are rebuilt when recreating
                params.upgrade = true;
            } else {
                info!("{}", strings::USER_DECLINED_UPGRADE);
            }
        } else {
            info!("{}", strings::CONTAINER_UP_TO_DATE);
        }
    }

    if !jail_exists || should_recreate {
        if jail_exists && should_recreate {
            if params.auth && !params.upgrade && params.force_layers.is_empty() {
                // Auth-only recreation (no layer rebuilding)
                info!(
                    "Recreating jail '{}' with host networking for authentication (layers will not be rebuilt)...",
                    jail_name
                );
            } else if params.upgrade || !params.force_layers.is_empty() {
                info!(
                    "{}",
                    strings::format_string(strings::RECREATING_JAIL_UPGRADE, &jail_name)
                );
            } else {
                info!(
                    "{}",
                    strings::format_string(strings::RECREATING_JAIL_DETECTED_UPDATES, &jail_name)
                );
            }

            // Manually remove the container before recreating
            // This allows us to avoid setting upgrade=true when we only want to recreate
            // the container (e.g., for network mode changes) without rebuilding layers
            info!(
                "Stopping and removing existing container '{}'...",
                jail_name
            );
            temp_jail.remove(false).await?; // false = don't remove volume (preserve data)
        }

        // Only use custom image if explicitly provided (not default)
        // This allows the layered image system to auto-detect and build agent-specific images
        let use_custom_image = params.image != crate::cli::DEFAULT_IMAGE;

        let mut builder = JailBuilder::new(&jail_name).backend(backend_type);

        // Configure network mode based on flags
        if params.auth {
            // OAuth authentication requires host networking for callbacks
            builder = builder.host_network(true);
            info!("Using host networking for OAuth authentication");
        } else {
            // Normal operation: use private or shared networking
            builder = builder.network(!params.no_network, true);
        }

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
        // Special handling for git worktrees
        if !params.no_workspace {
            let workspace_dir = get_git_root().unwrap_or_else(|| cwd.clone());

            // Validate workspace directory is safe
            validate_workspace_directory(&workspace_dir)?;

            // Check if this is a git worktree
            if let Some(worktree_info) = crate::worktree::detect_worktree(&workspace_dir)? {
                info!("Detected git worktree, setting up dual-mount configuration");

                // Collect paths that need parent directory creation
                let paths_to_mount = vec![
                    worktree_info.worktree_path.as_path(),
                    worktree_info.main_git_dir.as_path(),
                ];
                let parent_dirs = crate::worktree::get_required_parent_dirs(&paths_to_mount);

                info!(
                    "Will create {} parent directories in container",
                    parent_dirs.len()
                );
                builder = builder.pre_create_dirs(parent_dirs);

                // Mount 1: Worktree at /workspace (familiar location)
                info!(
                    "Mounting worktree {} to /workspace",
                    worktree_info.worktree_path.display()
                );
                builder = builder.bind_mount(&worktree_info.worktree_path, "/workspace", false);

                // Mount 2: Worktree at original absolute path (preserve .git file reference)
                info!(
                    "Mounting worktree {} to {} (preserve absolute path)",
                    worktree_info.worktree_path.display(),
                    worktree_info.worktree_path.display()
                );
                builder = builder.bind_mount(
                    &worktree_info.worktree_path,
                    &worktree_info.worktree_path,
                    false,
                );

                // Mount 3: Main .git at original absolute path (read-write for git operations)
                info!(
                    "Mounting main git directory {} to {} (read-write)",
                    worktree_info.main_git_dir.display(),
                    worktree_info.main_git_dir.display()
                );
                builder = builder.bind_mount(
                    &worktree_info.main_git_dir,
                    &worktree_info.main_git_dir,
                    false,
                );
            } else {
                // Regular directory, not a worktree
                info!(
                    "Auto-mounting {} to {}",
                    workspace_dir.display(),
                    params.workspace_path
                );
                builder = builder.bind_mount(workspace_dir, params.workspace_path, false);
            }
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
                claude_code_router_dir: params.claude_code_router_dir,
                copilot_dir: params.copilot_dir,
                cursor_dir: params.cursor_dir,
                gemini_dir: params.gemini_dir,
                codex_dir: params.codex_dir,
                jules_dir: params.jules_dir,
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

        // Parse port mappings
        for port_str in params.port {
            let port_mapping = Commands::parse_port(&port_str).map_err(error::JailError::Config)?;
            builder = builder.port_mapping(
                port_mapping.host_port,
                port_mapping.container_port,
                &port_mapping.protocol,
            );
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

        // Set upgrade flag - this controls whether layers are rebuilt
        // Container removal is handled manually above when should_recreate is true
        builder = builder.upgrade(params.upgrade);

        // Set force layers
        builder = builder.force_layers(params.force_layers);

        // Set isolated flag
        builder = builder.isolated(params.isolated);

        // Set verbose flag
        builder = builder.verbose(params.verbose);

        // Set no_nix flag
        builder = builder.no_nix(params.no_nix);

        // Set block_host flag (inverted: !no_block_host means blocking is enabled)
        builder = builder.block_host(!params.no_block_host);

        let jail = builder.build();
        jail.create().await?;

        // Create .claude.json file inside the container for Claude agent
        if let Some(agent) = crate::agents::Agent::from_str(agent_command) {
            if agent == crate::agents::Agent::Claude {
                if let Err(e) = create_claude_json_in_container(&home_path, &jail).await {
                    tracing::warn!("Failed to create .claude.json in container: {}", e);
                }
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

    // Handle OAuth authentication workflow if --auth flag is provided
    if params.auth {
        return handle_auth_workflow(agent_command, &jail_name, backend_type).await;
    }

    info!("about to run");

    // Check if the jail uses the Nix wrapper by testing if it exists in the agent's home directory
    // If it does, wrap the command with nix-wrapper to ensure flake environment is loaded
    // Use a simple file check that's fast and won't hang
    let wrapper_check = jail
        .exec(
            &[
                "sh".to_string(),
                "-c".to_string(),
                "[ -x /usr/local/bin/nix-wrapper ] && echo yes || echo no".to_string(),
            ],
            false,
        )
        .await;
    let uses_nix_wrapper = wrapper_check
        .map(|output| output.trim() == "yes")
        .unwrap_or(false);

    // Check if this agent requires a server to be started first
    if let Some(agent) = crate::agents::Agent::from_str(agent_command) {
        if agent.requires_server_start() {
            info!("Agent requires server start, executing startup sequence");
            
            if let Some(server_cmd) = agent.server_start_command() {
                // Start the server in the background
                info!("Starting {} server with command: {} {}", agent.display_name(), agent_command, server_cmd);
                
                let server_command = if uses_nix_wrapper {
                    vec![
                        "/usr/local/bin/nix-wrapper".to_string(),
                        agent_command.to_string(),
                        server_cmd.to_string(),
                    ]
                } else {
                    vec![agent_command.to_string(), server_cmd.to_string()]
                };
                
                // Execute server start in background (non-interactive)
                let server_output = jail.exec(&server_command, false).await?;
                if !server_output.is_empty() {
                    info!("Server start output: {}", server_output);
                }
                
                // Wait a bit for server to start
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                info!("Server started, proceeding with main command");
            }
            
            // Override agent_command with the main command for agents that require server start
            if let Some(main_cmd) = agent.main_command() {
                let command = if uses_nix_wrapper {
                    let mut cmd = vec![
                        "/usr/local/bin/nix-wrapper".to_string(),
                        agent_command.to_string(),
                        main_cmd.to_string(),
                    ];
                    cmd.extend(params.args);
                    cmd
                } else {
                    let mut cmd = vec![agent_command.to_string(), main_cmd.to_string()];
                    cmd.extend(params.args);
                    cmd
                };
                
                let output = jail.exec(&command, true).await?;
                if !output.is_empty() {
                    print!("{output}");
                }
                
                return Ok(());
            }
        }
    }

    let command = if uses_nix_wrapper {
        // Wrap command with nix-wrapper to load flake environment
        let mut cmd = vec![
            "/usr/local/bin/nix-wrapper".to_string(),
            agent_command.to_string(),
        ];
        cmd.extend(params.args);
        cmd
    } else {
        // No Nix wrapper available, execute command directly
        let mut cmd = vec![agent_command.to_string()];
        cmd.extend(params.args);
        cmd
    };

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

/// Handle OAuth authentication workflow for agents that support it
/// This opens an interactive shell in the container with network=host for OAuth callbacks
async fn handle_auth_workflow(
    agent_command: &str,
    jail_name: &str,
    backend_type: crate::config::BackendType,
) -> Result<()> {
    // Check if this agent supports the auth workflow
    let agent = crate::agents::Agent::from_str(agent_command)
        .ok_or_else(|| error::JailError::Config(format!("Unknown agent: {}", agent_command)))?;

    if !agent.supports_auth_workflow() {
        return Err(error::JailError::Config(format!(
            "Agent '{}' does not support the --auth workflow. Only Codex and Jules agents support this feature.",
            agent.display_name()
        )));
    }

    info!(
        "Starting OAuth authentication workflow for {}",
        agent.display_name()
    );

    // Get agent-specific auth command
    let auth_command = match agent {
        crate::agents::Agent::Codex => "codex auth login",
        crate::agents::Agent::Jules => "jules auth",
        _ => "auth command", // Fallback for future agents
    };

    // Create backend to check container state
    let temp_config = crate::config::JailConfig {
        name: jail_name.to_string(),
        backend: backend_type,
        ..Default::default()
    };
    let backend = crate::backend::create_backend(&temp_config);

    // Check if container exists
    let container_exists = backend.exists(jail_name).await?;

    if !container_exists {
        return Err(error::JailError::NotFound(format!(
            "Container '{}' does not exist. Please create it first by running: jail-ai {} [options]",
            jail_name, agent_command
        )));
    }

    // Check if container is running
    let is_running = backend.is_running(jail_name).await?;

    // Note: Container should already have been recreated with host networking
    // by the main flow when --auth flag was detected

    if is_running {
        // Container is already running, just join it with an interactive shell
        info!("Container is running with host networking. Opening shell for authentication...");
        println!("\n🔐 Authentication Mode - {}", agent.display_name());
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Opening interactive shell in container with host networking.");
        println!();
        println!("To authenticate, run:");
        println!("  {}", auth_command);
        println!();
        println!("After authentication is complete:");
        println!("  1. Exit the shell with 'exit' or Ctrl+D");
        println!("  2. Restart the container with: jail-ai {}", agent_command);
        println!("     (This will restore secure network isolation)");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        let jail = crate::jail::JailBuilder::new(jail_name)
            .backend(backend_type)
            .build();

        let output = jail.exec(&["/usr/bin/zsh".to_string()], true).await?;
        if !output.is_empty() {
            print!("{}", output);
        }

        println!("\n⚠️  Remember to restart the container to restore secure networking:");
        println!("   jail-ai {}", agent_command);
    } else {
        // Container is stopped, we need to start it
        info!("Container is stopped. Starting it for authentication...");

        println!("\n🔐 Authentication Mode - {}", agent.display_name());
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Starting container with host networking for OAuth authentication.");
        println!();
        println!("To authenticate, run:");
        println!("  {}", auth_command);
        println!();
        println!("After authentication is complete:");
        println!("  1. Exit the shell with 'exit' or Ctrl+D");
        println!("  2. Restart the container with: jail-ai {}", agent_command);
        println!("     (This will restore secure network isolation)");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        // Start the container
        backend.start(jail_name).await?;

        // Open interactive shell
        let jail = crate::jail::JailBuilder::new(jail_name)
            .backend(backend_type)
            .build();

        let output = jail.exec(&["/usr/bin/zsh".to_string()], true).await?;
        if !output.is_empty() {
            print!("{}", output);
        }

        println!("\n⚠️  Remember to restart the container to restore secure networking:");
        println!("   jail-ai {}", agent_command);
    }

    Ok(())
}

/// Extract agent name from jail name for display purposes
/// Jail name format: jail-{project}-{hash}-{agent}
/// Returns a simplified agent name for display (e.g., "cursor" instead of "cursor-agent")
pub fn extract_agent_name(jail_name: &str) -> &'static str {
    crate::agents::get_agent_display_name(jail_name)
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
    io::stdout()
        .flush()
        .map_err(|e| error::JailError::Config(format!("Failed to flush stdout: {e}")))?;

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
