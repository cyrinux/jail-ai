use clap::{Parser, Subcommand};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "jail-ai")]
#[command(about = "AI Agent Jail Manager - Sandbox AI agents using systemd-nspawn or podman", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Create a new jail
    Create {
        /// Name of the jail (auto-generated from current directory if not provided)
        name: Option<String>,

        /// Backend type (systemd-nspawn or podman)
        #[arg(short, long, default_value = "podman")]
        backend: String,

        /// Base image (e.g., localhost/jail-ai-env:latest, alpine:latest)
        #[arg(short, long, default_value = "localhost/jail-ai-env:latest")]
        image: String,

        /// Bind mount (format: source:target[:ro])
        #[arg(short = 'm', long)]
        mount: Vec<String>,

        /// Environment variable (format: KEY=VALUE)
        #[arg(short, long)]
        env: Vec<String>,

        /// Disable network access
        #[arg(long)]
        no_network: bool,

        /// Memory limit in MB
        #[arg(long)]
        memory: Option<u64>,

        /// CPU quota percentage (0-100)
        #[arg(long)]
        cpu: Option<u32>,

        /// Load configuration from file
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Skip auto-mounting current working directory to /workspace
        #[arg(long)]
        no_workspace: bool,

        /// Custom workspace path inside jail (default: /workspace)
        #[arg(long, default_value = "/workspace")]
        workspace_path: String,

        /// Skip auto-mounting AI agent config directories (~/.claude, ~/.config, ~/.cursor)
        #[arg(long)]
        no_agent_configs: bool,

        /// Disable auto-mounting git configuration
        #[arg(long)]
        no_git_config: bool,

        /// Disable auto-mounting GPG configuration
        #[arg(long)]
        no_gpg_config: bool,
    },

    /// Start a jail
    Start {
        /// Name of the jail
        name: String,
    },

    /// Stop a jail
    Stop {
        /// Name of the jail
        name: String,
    },

    /// Remove a jail
    Remove {
        /// Name of the jail
        name: String,

        /// Force removal without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Execute a command in a jail
    Exec {
        /// Name of the jail
        name: String,

        /// Run in interactive mode with TTY (default: true, use --no-interactive to disable)
        #[arg(short, long, default_value_t = true, action = clap::ArgAction::Set)]
        interactive: bool,

        /// Command to execute
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },

    /// Show jail status
    Status {
        /// Name of the jail
        name: String,
    },

    /// Save jail configuration to file
    Save {
        /// Name of the jail
        name: String,

        /// Output file path
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Quick start Claude Code in a jail for current directory
    Claude {
        /// Backend type (systemd-nspawn or podman)
        #[arg(short, long, default_value = "podman")]
        backend: String,

        /// Base image (e.g., localhost/jail-ai-env:latest, alpine:latest)
        #[arg(short, long, default_value = "localhost/jail-ai-env:latest")]
        image: String,

        /// Bind mount (format: source:target[:ro])
        #[arg(short = 'm', long)]
        mount: Vec<String>,

        /// Environment variable (format: KEY=VALUE)
        #[arg(short = 'e', long)]
        env: Vec<String>,

        /// Disable network access
        #[arg(long)]
        no_network: bool,

        /// Memory limit in MB
        #[arg(long)]
        memory: Option<u64>,

        /// CPU quota percentage (0-100)
        #[arg(long)]
        cpu: Option<u32>,

        /// Skip auto-mounting current working directory to /workspace
        #[arg(long)]
        no_workspace: bool,

        /// Custom workspace path inside jail (default: /workspace)
        #[arg(long, default_value = "/workspace")]
        workspace_path: String,

        /// Skip auto-mounting AI agent config directories (~/.claude, ~/.config, ~/.cursor)
        #[arg(long)]
        no_agent_configs: bool,

        /// Disable auto-mounting git configuration
        #[arg(long)]
        no_git_config: bool,

        /// Disable auto-mounting GPG configuration
        #[arg(long)]
        no_gpg_config: bool,

        /// Additional arguments to pass to claude
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Quick start GitHub Copilot CLI in a jail for current directory
    Copilot {
        /// Backend type (systemd-nspawn or podman)
        #[arg(short, long, default_value = "podman")]
        backend: String,

        /// Base image (e.g., localhost/jail-ai-env:latest, alpine:latest)
        #[arg(short, long, default_value = "localhost/jail-ai-env:latest")]
        image: String,

        /// Bind mount (format: source:target[:ro])
        #[arg(short = 'm', long)]
        mount: Vec<String>,

        /// Environment variable (format: KEY=VALUE)
        #[arg(short = 'e', long)]
        env: Vec<String>,

        /// Disable network access
        #[arg(long)]
        no_network: bool,

        /// Memory limit in MB
        #[arg(long)]
        memory: Option<u64>,

        /// CPU quota percentage (0-100)
        #[arg(long)]
        cpu: Option<u32>,

        /// Skip auto-mounting current working directory to /workspace
        #[arg(long)]
        no_workspace: bool,

        /// Custom workspace path inside jail (default: /workspace)
        #[arg(long, default_value = "/workspace")]
        workspace_path: String,

        /// Skip auto-mounting AI agent config directories (~/.claude, ~/.config, ~/.cursor)
        #[arg(long)]
        no_agent_configs: bool,

        /// Disable auto-mounting git configuration
        #[arg(long)]
        no_git_config: bool,

        /// Disable auto-mounting GPG configuration
        #[arg(long)]
        no_gpg_config: bool,

        /// Additional arguments to pass to copilot
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Quick start Cursor Agent in a jail for current directory
    Cursor {
        /// Backend type (systemd-nspawn or podman)
        #[arg(short, long, default_value = "podman")]
        backend: String,

        /// Base image (e.g., localhost/jail-ai-env:latest, alpine:latest)
        #[arg(short, long, default_value = "localhost/jail-ai-env:latest")]
        image: String,

        /// Bind mount (format: source:target[:ro])
        #[arg(short = 'm', long)]
        mount: Vec<String>,

        /// Environment variable (format: KEY=VALUE)
        #[arg(short = 'e', long)]
        env: Vec<String>,

        /// Disable network access
        #[arg(long)]
        no_network: bool,

        /// Memory limit in MB
        #[arg(long)]
        memory: Option<u64>,

        /// CPU quota percentage (0-100)
        #[arg(long)]
        cpu: Option<u32>,

        /// Skip auto-mounting current working directory to /workspace
        #[arg(long)]
        no_workspace: bool,

        /// Custom workspace path inside jail (default: /workspace)
        #[arg(long, default_value = "/workspace")]
        workspace_path: String,

        /// Skip auto-mounting AI agent config directories (~/.claude, ~/.config, ~/.cursor)
        #[arg(long)]
        no_agent_configs: bool,

        /// Disable auto-mounting git configuration
        #[arg(long)]
        no_git_config: bool,

        /// Disable auto-mounting GPG configuration
        #[arg(long)]
        no_gpg_config: bool,

        /// Additional arguments to pass to cursor-agent
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Join an interactive shell in the jail for current directory
    Join {
        /// Shell to use (default: bash)
        #[arg(short, long, default_value = "bash")]
        shell: String,
    },

    /// Stop and remove all jail-ai containers
    CleanAll {
        /// Backend type to clean (systemd-nspawn or podman, default: both)
        #[arg(short, long)]
        backend: Option<String>,

        /// Force removal without confirmation
        #[arg(short, long)]
        force: bool,
    },
}

impl Commands {
    pub fn parse_backend(backend: &str) -> Result<crate::config::BackendType, String> {
        match backend.to_lowercase().as_str() {
            "systemd-nspawn" | "nspawn" | "systemd" => {
                Ok(crate::config::BackendType::SystemdNspawn)
            }
            "podman" | "pod" => Ok(crate::config::BackendType::Podman),
            "docker" => Ok(crate::config::BackendType::Docker),
            _ => Err(format!(
                "Invalid backend '{}'. Supported: systemd-nspawn, podman, docker",
                backend
            )),
        }
    }

    pub fn parse_mount(mount_str: &str) -> Result<crate::config::BindMount, String> {
        let parts: Vec<&str> = mount_str.split(':').collect();
        if parts.len() < 2 {
            return Err(format!(
                "Invalid mount format '{}'. Expected: source:target[:ro]",
                mount_str
            ));
        }

        let readonly = parts.get(2).is_some_and(|&s| s == "ro");

        Ok(crate::config::BindMount {
            source: PathBuf::from(parts[0]),
            target: PathBuf::from(parts[1]),
            readonly,
        })
    }

    pub fn parse_env(env_str: &str) -> Result<(String, String), String> {
        let parts: Vec<&str> = env_str.splitn(2, '=').collect();
        if parts.len() != 2 {
            return Err(format!(
                "Invalid environment variable format '{}'. Expected: KEY=VALUE",
                env_str
            ));
        }

        Ok((parts[0].to_string(), parts[1].to_string()))
    }

    /// Sanitize a jail name to match podman/systemd-nspawn requirements
    /// Names must match [a-zA-Z0-9][a-zA-Z0-9_.-]*
    pub fn sanitize_jail_name(name: &str) -> String {
        // Strip leading non-alphanumeric characters (dots, hyphens, etc.)
        let name = name.trim_start_matches(|c: char| !c.is_alphanumeric());

        if name.is_empty() {
            return "default".to_string();
        }

        // Replace invalid characters with hyphens
        // Valid characters after first: alphanumeric, underscore, dot, hyphen
        let sanitized: String = name
            .chars()
            .enumerate()
            .map(|(i, c)| {
                if i == 0 {
                    // First character must be alphanumeric
                    if c.is_alphanumeric() {
                        c
                    } else {
                        'x'
                    }
                } else {
                    // Remaining characters can be alphanumeric, _, ., or -
                    if c.is_alphanumeric() || c == '_' || c == '.' || c == '-' {
                        c
                    } else {
                        '-'
                    }
                }
            })
            .collect();

        if sanitized.is_empty() {
            "default".to_string()
        } else {
            sanitized
        }
    }

    /// Generate a reproducible container name from a directory path
    pub fn generate_jail_name(path: &std::path::Path) -> String {
        // Get the absolute path and canonicalize it
        let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        // Create SHA-256 hash of the path
        let mut hasher = Sha256::new();
        hasher.update(abs_path.to_string_lossy().as_bytes());
        let hash = hasher.finalize();
        let hash_hex = hex::encode(hash);

        // Get the directory name for human readability
        let dir_name = abs_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("workspace");

        // Sanitize directory name
        let sanitized_name = Self::sanitize_jail_name(dir_name);

        // Use first 8 characters of hash for uniqueness
        format!("jail-{}-{}", sanitized_name, &hash_hex[..8])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_backend() {
        assert!(matches!(
            Commands::parse_backend("podman"),
            Ok(crate::config::BackendType::Podman)
        ));
        assert!(matches!(
            Commands::parse_backend("systemd-nspawn"),
            Ok(crate::config::BackendType::SystemdNspawn)
        ));
        assert!(Commands::parse_backend("invalid").is_err());
    }

    #[test]
    fn test_parse_mount() {
        let mount = Commands::parse_mount("/src:/dst:ro").unwrap();
        assert_eq!(mount.source, PathBuf::from("/src"));
        assert_eq!(mount.target, PathBuf::from("/dst"));
        assert!(mount.readonly);

        let mount = Commands::parse_mount("/src:/dst").unwrap();
        assert!(!mount.readonly);

        assert!(Commands::parse_mount("invalid").is_err());
    }

    #[test]
    fn test_parse_env() {
        let (key, value) = Commands::parse_env("KEY=VALUE").unwrap();
        assert_eq!(key, "KEY");
        assert_eq!(value, "VALUE");

        assert!(Commands::parse_env("INVALID").is_err());
    }

    #[test]
    fn test_sanitize_jail_name() {
        // Test dotfile names
        assert_eq!(Commands::sanitize_jail_name(".dotfiles"), "dotfiles");
        assert_eq!(Commands::sanitize_jail_name("...dotfiles"), "dotfiles");

        // Test names with special characters
        assert_eq!(Commands::sanitize_jail_name("my@project"), "my-project");
        assert_eq!(Commands::sanitize_jail_name("test project"), "test-project");

        // Test valid characters that should be preserved
        assert_eq!(
            Commands::sanitize_jail_name("my_project.v2"),
            "my_project.v2"
        );
        assert_eq!(
            Commands::sanitize_jail_name("my-project-v2"),
            "my-project-v2"
        );

        // Test leading hyphens/underscores
        assert_eq!(Commands::sanitize_jail_name("-myproject"), "myproject");
        assert_eq!(Commands::sanitize_jail_name("_myproject"), "myproject");

        // Test empty or all-invalid names
        assert_eq!(Commands::sanitize_jail_name("..."), "default");
        assert_eq!(Commands::sanitize_jail_name(""), "default");
        assert_eq!(Commands::sanitize_jail_name("---"), "default");

        // Test that first character must be alphanumeric
        assert_eq!(Commands::sanitize_jail_name(".project"), "project");

        // Test normal names remain unchanged
        assert_eq!(Commands::sanitize_jail_name("myproject"), "myproject");
        assert_eq!(Commands::sanitize_jail_name("MyProject123"), "MyProject123");
    }

    #[test]
    fn test_generate_jail_name() {
        use std::path::PathBuf;

        // Test with a simple path
        let path = PathBuf::from("/tmp/test-project");
        let name = Commands::generate_jail_name(&path);

        // Should start with "jail-"
        assert!(name.starts_with("jail-"));

        // Should contain sanitized directory name
        assert!(name.contains("test-project"));

        // Should be reproducible - same path generates same name
        let name2 = Commands::generate_jail_name(&path);
        assert_eq!(name, name2);

        // Different paths should generate different names
        let path2 = PathBuf::from("/tmp/another-project");
        let name3 = Commands::generate_jail_name(&path2);
        assert_ne!(name, name3);
    }

    #[test]
    fn test_generate_jail_name_sanitization() {
        use std::path::PathBuf;

        // Test with special characters in path
        let path = PathBuf::from("/tmp/my-project@2024");
        let name = Commands::generate_jail_name(&path);

        // Special characters should be sanitized to hyphens
        assert!(name.contains("my-project-2024"));
    }
}
