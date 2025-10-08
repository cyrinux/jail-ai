use crate::error::{JailError, Result};
use crate::jail::{JailBuilder, JailManager};
use crate::jail_setup::{copy_dir_recursive, get_jail_ai_config_dir, get_user_uid};
use std::path::Path;
use tracing::{debug, info, warn};

/// Get git config value with fallback to global and system config
///
/// This function respects git's config precedence hierarchy:
/// - **Local** (`.git/config` in repository) - highest priority
/// - **Global** (`~/.gitconfig` or `~/.config/git/config`) - medium priority
/// - **System** (`/etc/gitconfig`) - lowest priority
///
/// When multiple values exist for the same key, git uses the **last value** from the
/// **highest priority scope**. For example, if a key has multiple values in local config,
/// the last one wins. If a key exists in both local and global config, local wins.
///
/// # Fallback Strategy
///
/// The function tries these operations in order:
/// 1. **`--local` with cwd context**: Reads only the repository's `.git/config`
///    - Returns immediately if found (optimization for most common case)
/// 2. **No scope with cwd context**: Reads local + global + system in precedence order
///    - Handles cases where local doesn't exist but global/system does
/// 3. **`--global` without cwd**: Reads only global config
///    - Fallback for when cwd is not a git repository
/// 4. **`--system` without cwd**: Reads only system config
///    - Final fallback for system-level configuration
///
/// The fallback chain ensures correct behavior both inside and outside git repositories.
///
/// # Handling Multiple Values
///
/// Git allows multiple values for the same key (using `git config --add`). This function:
/// - Uses `--get-all` to retrieve all values for a key
/// - Uses `next_back()` to get the **last value** (git's documented behavior)
/// - This correctly implements git's "last value wins" semantics
///
/// # Examples
///
/// ```text
/// # Inside a git repository
/// Local:  user.name = "Alice"
/// Global: user.name = "Bob"
/// Result: "Alice" (local wins)
///
/// # Outside a git repository
/// Global: user.name = "Bob"
/// Result: "Bob" (fallback to global)
///
/// # Multiple values in same scope
/// Local:  user.name = "Alice"
/// Local:  user.name = "Charlie"
/// Result: "Charlie" (last value wins)
/// ```
pub fn get_git_config(key: &str, cwd: &Path) -> Option<String> {
    /// Helper to try reading git config with specific args
    ///
    /// Uses `--get-all` to retrieve all values and returns the last one
    /// (which has highest priority according to git's semantics)
    fn try_git_config(args: &[&str], key: &str, scope: &str, cwd: Option<&Path>) -> Option<String> {
        let mut cmd = std::process::Command::new("git");
        if let Some(cwd) = cwd {
            cmd.current_dir(cwd);
        }

        let output = cmd.args(args).output().ok()?;

        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            // Get the last non-empty line (git uses last value when there are duplicates)
            // This correctly implements git's "last value wins" behavior
            if let Some(value) = output_str
                .lines()
                .filter(|l| !l.trim().is_empty())
                .next_back()
            {
                let value = value.trim().to_string();
                debug!("Found {} in {} config: {}", key, scope, value);
                return Some(value);
            }
        }

        None
    }

    // Try project-specific config first (local to the repository)
    // Use --get-all to handle duplicate entries and take the last one
    // This is an optimization - if found in local, we don't need to check other scopes
    if let Some(value) = try_git_config(
        &["config", "--local", "--get-all", key],
        key,
        "local",
        Some(cwd),
    ) {
        return Some(value);
    }

    // Try project config (no scope specified - reads local + global + system in precedence order)
    // This handles the case where local doesn't exist but global/system does
    if let Some(value) = try_git_config(&["config", "--get-all", key], key, "project", Some(cwd)) {
        return Some(value);
    }

    // Fallback to global config only
    // This handles the case where cwd is not a git repository
    if let Some(value) = try_git_config(
        &["config", "--global", "--get-all", key],
        key,
        "global",
        None,
    ) {
        return Some(value);
    }

    // Fallback to system config only
    // Final fallback for system-level configuration
    if let Some(value) = try_git_config(
        &["config", "--system", "--get-all", key],
        key,
        "system",
        None,
    ) {
        return Some(value);
    }

    debug!("No value found for {} in any git config", key);
    None
}

/// Read all relevant git config values from the host
/// Returns a HashMap of config keys and their values
pub fn get_all_git_config_values(cwd: &Path) -> std::collections::HashMap<String, String> {
    use std::collections::HashMap;

    let mut config_map = HashMap::new();

    // List of git config keys we want to read
    let config_keys = vec![
        "user.name",
        "user.email",
        "user.signingkey",
        "commit.gpgsign",
        "tag.gpgsign",
        "gpg.format",
        "gpg.program",
        "gpg.ssh.allowedsignersfile",
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
pub fn generate_gitconfig_content(
    config_map: &std::collections::HashMap<String, String>,
) -> String {
    use std::collections::HashMap;

    let mut content = String::from("# Generated by jail-ai from host git config\n\n");

    // Group config by section and subsection
    // Key format: section.subsection.name or section.name
    let mut sections: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut subsections: HashMap<(String, String), Vec<(String, String)>> = HashMap::new();

    for (key, value) in config_map {
        let parts: Vec<&str> = key.split('.').collect();

        if parts.len() == 2 {
            // Simple: section.name
            let section = parts[0];
            let name = parts[1];
            sections
                .entry(section.to_string())
                .or_default()
                .push((name.to_string(), value.clone()));
        } else if parts.len() == 3 {
            // Subsection: section.subsection.name
            let section = parts[0];
            let subsection = parts[1];
            let name = parts[2];
            subsections
                .entry((section.to_string(), subsection.to_string()))
                .or_default()
                .push((name.to_string(), value.clone()));
        }
    }

    // Write sections in order
    let section_order = vec![
        "user", "commit", "tag", "gpg", "core", "init", "pull", "push",
    ];

    for section_name in section_order {
        // Write simple section entries
        if let Some(entries) = sections.get(section_name) {
            content.push_str(&format!("[{section_name}]\n"));
            for (name, value) in entries {
                content.push_str(&format!("\t{name} = {value}\n"));
            }
            content.push('\n');
        }

        // Write subsection entries for this section
        for ((sec, subsec), entries) in &subsections {
            if sec == section_name {
                content.push_str(&format!("[{sec} \"{subsec}\"]\n"));
                for (name, value) in entries {
                    content.push_str(&format!("\t{name} = {value}\n"));
                }
                content.push('\n');
            }
        }
    }

    // Add GitHub credential helper configuration
    content.push_str("[credential \"https://gist.github.com\"]\n");
    content.push_str("\thelper = \"\"\n");
    content.push_str("\thelper = \"gh auth git-credential\"\n");
    content.push('\n');

    content.push_str("[credential \"https://github.com\"]\n");
    content.push_str("\thelper = \"\"\n");
    content.push_str("\thelper = \"gh auth git-credential\"\n");
    content.push('\n');

    content
}

/// Handle SSH allowed signers file mounting for SSH-based GPG signing
/// Returns the updated builder and true if SSH GPG signing is configured and file was mounted
pub fn handle_ssh_allowed_signers_mounting(
    cwd: &Path,
    builder: &JailBuilder,
) -> Result<(JailBuilder, bool)> {
    // Read git config to check for SSH GPG configuration
    let config_map = get_all_git_config_values(cwd);

    if let Some(gpg_format) = config_map.get("gpg.format") {
        // Handle both quoted and unquoted values: "ssh" or ssh
        let format_value = gpg_format.trim_matches('"');
        if format_value == "ssh" {
            if let Some(allowedsigners_file) = config_map.get("gpg.ssh.allowedsignersfile") {
                // Handle both quoted and unquoted values: "~/.ssh/allowed_signers" or ~/.ssh/allowed_signers
                let file_path = allowedsigners_file.trim_matches('"');
                // Expand ~ to home directory
                let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
                let expanded_path = file_path.replace("~", &home);
                let signers_path = std::path::PathBuf::from(&expanded_path);

                if signers_path.exists() {
                    info!(
                        "Mounting SSH allowed signers file: {} to /home/agent/.ssh/allowed_signers",
                        signers_path.display()
                    );
                    let updated_builder = builder.clone().bind_mount(
                        &signers_path,
                        "/home/agent/.ssh/allowed_signers",
                        false,
                    );
                    return Ok((updated_builder, true));
                } else {
                    warn!("SSH allowed signers file not found: {} - SSH GPG signing may not work properly", signers_path.display());
                }
            } else {
                warn!("SSH GPG format configured but gpg.ssh.allowedsignersfile not set - SSH GPG signing may not work properly");
            }
        }
    }

    Ok((builder.clone(), false))
}

/// Prepare GPG configuration by resolving symlinks and copying to a persistent directory
/// This handles NixOS where config files are symlinks to /nix/store
/// Returns (persistent_dir_path, sockets_to_mount) where sockets need to be mounted separately
pub fn prepare_gpg_config(gpg_dir: &Path) -> Result<(std::path::PathBuf, Vec<std::path::PathBuf>)> {
    use std::os::unix::fs::FileTypeExt;

    if !gpg_dir.exists() {
        return Err(JailError::Config(format!(
            "GPG directory does not exist: {}",
            gpg_dir.display()
        )));
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
    info!(
        "Preparing GPG config in cache directory: {}",
        persistent_dir.display()
    );
    debug!("Source GPG directory: {}", gpg_dir.display());

    let mut sockets = Vec::new();

    // Look for GPG agent sockets in the runtime directory (/run/user/UID/gnupg/)
    // This is where gpg-agent actually creates the sockets
    let uid = get_user_uid()?;
    let runtime_gpg_dir = std::path::PathBuf::from(format!("/run/user/{uid}/gnupg"));

    if runtime_gpg_dir.exists() {
        debug!(
            "Checking runtime GPG directory: {}",
            runtime_gpg_dir.display()
        );

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
        debug!(
            "Runtime GPG directory does not exist: {}",
            runtime_gpg_dir.display()
        );
    }

    // Also check ~/.gnupg for any sockets (fallback)
    for entry in std::fs::read_dir(gpg_dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();

        // Skip GPG lockfiles (.#lk0*)
        if file_name_str.starts_with(".#lk0") {
            debug!("Skipping GPG lockfile: {}", file_name_str);
            continue;
        }

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
            if !sockets
                .iter()
                .any(|s| s.file_name() == Some(file_name.as_os_str()))
            {
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
                info!(
                    "Resolving GPG config symlink: {} -> {}",
                    file_name_str,
                    symlink_target.display()
                );
                let content = std::fs::read(&path)?;
                let content_len = content.len();
                std::fs::write(&target_path, content)?;
                debug!(
                    "Copied {} bytes from resolved symlink {} to cache",
                    content_len, file_name_str
                );
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

    info!(
        "Prepared GPG config in persistent cache directory: {}",
        persistent_dir.display()
    );
    debug!("Found {} GPG agent sockets to mount", sockets.len());
    Ok((persistent_dir, sockets))
}

/// Setup Git and GPG configuration for a jail
pub fn setup_git_gpg_config(
    mut builder: JailBuilder,
    cwd: &Path,
    home_path: &Path,
) -> Result<JailBuilder> {
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
                let mut ssh_auth_sock_target: Option<String> = None;
                for socket_path in sockets {
                    if let Some(socket_name) = socket_path.file_name() {
                        let socket_name_str = socket_name.to_string_lossy();
                        let target = format!("/home/agent/.gnupg/{socket_name_str}");
                        info!(
                            "Mounting GPG socket {} to {}",
                            socket_path.display(),
                            target
                        );
                        builder = builder.bind_mount(&socket_path, target.clone(), false);

                        // Track SSH auth socket for environment variable
                        if socket_name_str == "S.gpg-agent.ssh" {
                            ssh_auth_sock_target = Some(target);
                        }
                    }
                }

                // Set SSH_AUTH_SOCK if we mounted the SSH socket
                if let Some(sock_path) = ssh_auth_sock_target {
                    info!("Setting SSH_AUTH_SOCK to {}", sock_path);
                    builder = builder.env("SSH_AUTH_SOCK", sock_path);
                }
            }
            Err(e) => {
                warn!("Failed to prepare GPG config: {}", e);
            }
        }
    }

    // Handle SSH allowed signers file mounting for SSH-based GPG signing
    match handle_ssh_allowed_signers_mounting(cwd, &builder) {
        Ok((updated_builder, _mounted)) => {
            builder = updated_builder;
        }
        Err(e) => {
            warn!("Failed to handle SSH allowed signers file mounting: {}", e);
        }
    }

    Ok(builder)
}

/// Create a .gitconfig file inside the container
/// Reads git config from host and creates the file directly inside the container
pub async fn create_gitconfig_in_container(cwd: &Path, jail: &JailManager) -> Result<()> {
    // Read all relevant git config values from host
    let config_map = get_all_git_config_values(cwd);

    if config_map.is_empty() {
        debug!("No git config values found on host, skipping .gitconfig creation");
        return Ok(());
    }

    // Note: SSH allowed signers file mounting is handled during jail creation
    // This function only creates the .gitconfig file

    // Generate .gitconfig content
    let gitconfig_content = generate_gitconfig_content(&config_map);
    debug!("Generated .gitconfig content:\n{}", gitconfig_content);

    // Create the .gitconfig file inside the container using a shell command
    let create_file_cmd = vec![
        "sh".to_string(),
        "-c".to_string(),
        format!(
            "cat > /home/agent/.gitconfig << 'GITCONFIG_EOF'\n{}\nGITCONFIG_EOF",
            gitconfig_content
        ),
    ];

    jail.exec(&create_file_cmd, false).await?;

    info!("Created .gitconfig inside container with host's git configuration");

    Ok(())
}

/// Create a .claude.json file inside the container
/// Reads oauthAccount and userID from host's ~/.claude.json if it exists
/// Creates the file directly inside the container, not as a mount
pub async fn create_claude_json_in_container(home_path: &Path, jail: &JailManager) -> Result<()> {
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
