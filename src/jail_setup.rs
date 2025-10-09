use crate::error::Result;
use crate::jail::JailBuilder;
use std::path::Path;
use tracing::{info, warn};

/// Get the host's timezone
pub fn get_host_timezone() -> Option<String> {
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

/// Setup default environment variables for a jail
pub fn setup_default_environment(builder: JailBuilder) -> JailBuilder {
    let mut builder = builder;

    // Set timezone from host
    if let Some(tz) = get_host_timezone() {
        builder = builder.env("TZ", tz);
    }

    // Inherit TERM from host
    if let Ok(term) = std::env::var("TERM") {
        builder = builder.env("TERM", term);
    }

    // Set default editor to vim
    builder = builder.env("EDITOR", "vim");

    builder
}

/// Agent configuration flags
pub struct AgentConfigFlags {
    pub claude_dir: bool,
    pub copilot_dir: bool,
    pub cursor_dir: bool,
    pub gemini_dir: bool,
    pub codex_dir: bool,
    pub agent_configs: bool,
}

/// Helper function to mount a config directory if it exists
fn mount_config_if_exists(
    builder: JailBuilder,
    source_path: std::path::PathBuf,
    target_path: &str,
) -> JailBuilder {
    if source_path.exists() {
        info!("Mounting {} to {}", source_path.display(), target_path);
        builder.bind_mount(source_path, target_path, false)
    } else {
        builder
    }
}

/// Mount agent configuration directories based on flags
pub fn mount_agent_configs(
    builder: JailBuilder,
    home_path: &Path,
    agent: &str,
    flags: &AgentConfigFlags,
) -> JailBuilder {
    let mut builder = builder;

    // Opt-in: Mount entire ~/.claude directory
    if flags.claude_dir || flags.agent_configs {
        builder = mount_config_if_exists(builder, home_path.join(".claude"), "/home/agent/.claude");
    } else if agent == "claude" {
        // If not mounting full .claude directory, mount minimal auth files for Claude agent only
        let claude_creds = home_path.join(".claude").join(".credentials.json");
        if claude_creds.exists() {
            info!(
                "Auto-mounting {} to /home/agent/.claude/.credentials.json",
                claude_creds.display()
            );
            builder =
                builder.bind_mount(claude_creds, "/home/agent/.claude/.credentials.json", false);
        }
    }

    // Opt-in: Mount ~/.config/.copilot for GitHub Copilot
    if flags.copilot_dir || flags.agent_configs {
        builder = mount_config_if_exists(
            builder,
            home_path.join(".config").join(".copilot"),
            "/home/agent/.config/.copilot",
        );
    }

    // Opt-in: Mount ~/.cursor and ~/.config/cursor for Cursor Agent
    if flags.cursor_dir || flags.agent_configs {
        // Mount ~/.cursor (contains: chats, extensions, projects, cli-config.json, etc.)
        builder = mount_config_if_exists(builder, home_path.join(".cursor"), "/home/agent/.cursor");

        // Mount ~/.config/cursor (contains: auth.json, cli-config.json, prompt_history.json, etc.)
        builder = mount_config_if_exists(
            builder,
            home_path.join(".config").join("cursor"),
            "/home/agent/.config/cursor",
        );
    }

    // Opt-in: Mount ~/.config/gemini for Gemini CLI
    if flags.gemini_dir || flags.agent_configs {
        builder = mount_config_if_exists(
            builder,
            home_path.join(".config").join("gemini"),
            "/home/agent/.config/gemini",
        );
    }

    // Opt-in: Mount ~/.codex for Codex CLI
    if flags.codex_dir || flags.agent_configs {
        builder = mount_config_if_exists(builder, home_path.join(".codex"), "/home/agent/.codex");
    }

    builder
}

/// Get the user's UID for runtime directory detection
#[cfg(unix)]
pub fn get_user_uid() -> Result<u32> {
    use std::os::unix::fs::MetadataExt;
    let home = std::env::var("HOME").map_err(|_| {
        crate::error::JailError::Config("HOME environment variable not set".to_string())
    })?;
    let metadata = std::fs::metadata(&home)?;
    Ok(metadata.uid())
}

#[cfg(not(unix))]
pub fn get_user_uid() -> Result<u32> {
    Err(crate::error::JailError::Config(
        "UID detection not supported on non-Unix systems".to_string(),
    ))
}

/// Get the jail-ai config directory path (XDG_CONFIG_HOME or ~/.config/jail-ai)
pub fn get_jail_ai_config_dir() -> Result<std::path::PathBuf> {
    let base_dir = if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
        std::path::PathBuf::from(config_home)
    } else if let Ok(home) = std::env::var("HOME") {
        std::path::PathBuf::from(home).join(".config")
    } else {
        return Err(crate::error::JailError::Config(
            "Could not determine config directory (HOME not set)".to_string(),
        ));
    };

    Ok(base_dir.join("jail-ai"))
}

/// Recursively copy a directory
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    use tracing::debug;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();

        // Skip GPG lockfiles (.#lk0*)
        if file_name_str.starts_with(".#lk0") {
            debug!("Skipping GPG lockfile: {}", file_name_str);
            continue;
        }

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
