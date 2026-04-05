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

pub struct AgentConfigFlags {
    pub claude_dir: bool,
    pub claude_code_router_dir: bool,
    pub copilot_dir: bool,
    pub cursor_dir: bool,
    pub gemini_dir: bool,
    pub coderabbit_dir: bool,
    pub codex_dir: bool,
    pub jules_dir: bool,
    pub opencode_dir: bool,
    pub pi_dir: bool,
    pub agent_configs: bool,
}

impl AgentConfigFlags {
    pub fn to_agent_flags(&self) -> crate::agents::AgentConfigFlags {
        crate::agents::AgentConfigFlags {
            claude_dir: self.claude_dir,
            claude_code_router_dir: self.claude_code_router_dir,
            coderabbit_dir: self.coderabbit_dir,
            copilot_dir: self.copilot_dir,
            cursor_dir: self.cursor_dir,
            gemini_dir: self.gemini_dir,
            codex_dir: self.codex_dir,
            jules_dir: self.jules_dir,
            opencode_dir: self.opencode_dir,
            pi_dir: self.pi_dir,
            agent_configs: self.agent_configs,
        }
    }
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

pub fn mount_agent_configs(
    builder: JailBuilder,
    home_path: &Path,
    agent: &str,
    flags: &AgentConfigFlags,
) -> JailBuilder {
    let mut builder = builder;
    let agent_flags = flags.to_agent_flags();

    if let Some(parsed_agent) = crate::agents::Agent::from_str(agent) {
        let should_mount = parsed_agent.is_config_flag_set(&agent_flags);

        if should_mount {
            let config_paths = parsed_agent.config_dir_paths();
            for (host_path_str, container_path) in config_paths {
                let host_path = home_path.join(host_path_str);
                builder = mount_config_if_exists(builder, host_path, container_path);
            }
        } else if parsed_agent.has_auto_credentials() {
            let config_paths = parsed_agent.config_dir_paths();
            if let Some((host_path_str, container_path)) = config_paths.first() {
                let config_dir = home_path.join(host_path_str);
                let creds_file = config_dir.join(".credentials.json");
                if creds_file.exists() {
                    let target = format!("/home/agent/{}/.credentials.json", host_path_str);
                    info!("Auto-mounting {} to {}", creds_file.display(), target);
                    builder = builder.bind_mount(creds_file, target, false);
                } else if config_dir.exists() {
                    info!(
                        "Auto-mounting config dir {} to {}",
                        config_dir.display(),
                        container_path
                    );
                    builder = mount_config_if_exists(builder, config_dir, container_path);
                }
            }
        }
    } else {
        for agent_enum in crate::agents::ALL_AGENTS {
            if agent_enum.is_config_flag_set(&agent_flags) {
                let config_paths = agent_enum.config_dir_paths();
                for (host_path_str, container_path) in config_paths {
                    let host_path = home_path.join(host_path_str);
                    builder = mount_config_if_exists(builder, host_path, container_path);
                }
            }
        }
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
