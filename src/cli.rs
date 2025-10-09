use clap::{Args, Parser, Subcommand};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

/// Default image name
pub const DEFAULT_IMAGE: &str = "localhost/jail-ai-env:latest";

/// Common options for AI agent commands
#[derive(Args, Debug)]
pub struct AgentCommandOptions {
    /// Backend type (only 'podman' is supported, kept for compatibility)
    #[arg(short, long)]
    pub backend: Option<String>,

    /// Base image (e.g., localhost/jail-ai-env:latest, alpine:latest)
    #[arg(short, long, default_value = DEFAULT_IMAGE)]
    pub image: String,

    /// Bind mount (format: source:target[:ro])
    #[arg(short = 'm', long)]
    pub mount: Vec<String>,

    /// Environment variable (format: KEY=VALUE)
    #[arg(short = 'e', long)]
    pub env: Vec<String>,

    /// Disable network access
    #[arg(long)]
    pub no_network: bool,

    /// Memory limit in MB
    #[arg(long)]
    pub memory: Option<u64>,

    /// CPU quota percentage (0-100)
    #[arg(long)]
    pub cpu: Option<u32>,

    /// Skip auto-mounting current working directory to /workspace
    #[arg(long)]
    pub no_workspace: bool,

    /// Custom workspace path inside jail (default: /workspace)
    #[arg(long, default_value = "/workspace")]
    pub workspace_path: String,

    /// Mount entire ~/.claude directory (default: only .claude/.credentials.json)
    #[arg(long)]
    pub claude_dir: bool,

    /// Mount entire ~/.config directory for GitHub Copilot
    #[arg(long)]
    pub copilot_dir: bool,

    /// Mount entire ~/.cursor directory for Cursor Agent
    #[arg(long)]
    pub cursor_dir: bool,

    /// Mount entire ~/.config/gemini directory for Gemini CLI
    #[arg(long)]
    pub gemini_dir: bool,

    /// Mount entire ~/.config/codex directory for Codex CLI
    #[arg(long)]
    pub codex_dir: bool,

    /// Mount all agent config directories (combines --claude-dir, --copilot-dir, --cursor-dir, --gemini-dir, --codex-dir)
    #[arg(long)]
    pub agent_configs: bool,

    /// Enable git and GPG configuration mapping
    #[arg(long)]
    pub git_gpg: bool,

    /// Force rebuild of the default image, even if it already exists
    #[arg(long)]
    pub force_rebuild: bool,

    /// Force specific layers (comma-separated, e.g., "base,rust,python")
    #[arg(long, value_delimiter = ',')]
    pub layers: Vec<String>,

    /// Start an interactive shell instead of running the agent command
    #[arg(long)]
    pub shell: bool,

    /// Use isolated project-specific images (workspace hash tag) instead of shared layer-based images
    #[arg(long)]
    pub isolated: bool,

    /// API key for authentication (used with codex --auth)
    #[arg(long)]
    pub auth: Option<String>,
}

#[derive(Parser, Debug)]
#[command(name = "jail-ai")]
#[command(about = "AI Agent Jail Manager - Sandbox AI agents using podman", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Quiet mode (suppress INFO logs, only show warnings and errors)
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    pub quiet: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Create a new jail
    Create {
        /// Name of the jail (auto-generated from current directory if not provided)
        name: Option<String>,

        /// Backend type (only 'podman' is supported, kept for compatibility)
        #[arg(short, long)]
        backend: Option<String>,

        /// Base image (e.g., localhost/jail-ai-env:latest, alpine:latest)
        #[arg(short, long, default_value = DEFAULT_IMAGE)]
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

        /// Mount entire ~/.claude directory (default: only .claude/.credentials.json)
        #[arg(long)]
        claude_dir: bool,

        /// Mount entire ~/.config directory for GitHub Copilot
        #[arg(long)]
        copilot_dir: bool,

        /// Mount entire ~/.cursor directory for Cursor Agent
        #[arg(long)]
        cursor_dir: bool,

        /// Mount entire ~/.config/gemini directory for Gemini CLI
        #[arg(long)]
        gemini_dir: bool,

        /// Mount entire ~/.config/codex directory for Codex CLI
        #[arg(long)]
        codex_dir: bool,

        /// Mount all agent config directories (combines --claude-dir, --copilot-dir, --cursor-dir, --gemini-dir, --codex-dir)
        #[arg(long)]
        agent_configs: bool,

        /// Enable git and GPG configuration mapping
        #[arg(long)]
        git_gpg: bool,

        /// Force rebuild of the default image, even if it already exists
        #[arg(long)]
        force_rebuild: bool,

        /// Force specific layers (comma-separated, e.g., "base,rust,python")
        #[arg(long, value_delimiter = ',')]
        layers: Vec<String>,

        /// Use isolated project-specific images (workspace hash tag) instead of shared layer-based images
        #[arg(long)]
        isolated: bool,
    },

    /// Remove a jail
    Remove {
        /// Name of the jail (auto-detected from current directory if not provided)
        name: Option<String>,

        /// Force removal without confirmation
        #[arg(short, long)]
        force: bool,

        /// Remove associated volume (persistent data)
        #[arg(short, long)]
        volume: bool,
    },

    /// Show jail status
    Status {
        /// Name of the jail (auto-detected from current directory if not provided)
        name: Option<String>,
    },

    /// Save jail configuration to file
    Save {
        /// Name of the jail (auto-detected from current directory if not provided)
        name: Option<String>,

        /// Output file path
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Quick start Claude Code in a jail for current directory
    /// Use -- to separate jail-ai options from agent arguments
    /// Example: jail-ai claude --claude-dir -- --help
    Claude {
        #[command(flatten)]
        common: AgentCommandOptions,

        /// Additional arguments to pass to claude (after --)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Quick start GitHub Copilot CLI in a jail for current directory
    /// Use -- to separate jail-ai options from agent arguments
    /// Example: jail-ai copilot --copilot-dir -- suggest "write tests"
    Copilot {
        #[command(flatten)]
        common: AgentCommandOptions,

        /// Additional arguments to pass to copilot (after --)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Quick start Cursor Agent in a jail for current directory
    /// Use -- to separate jail-ai options from agent arguments
    /// Example: jail-ai cursor --cursor-dir -- --help
    Cursor {
        #[command(flatten)]
        common: AgentCommandOptions,

        /// Additional arguments to pass to cursor-agent (after --)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Quick start Gemini CLI in a jail for current directory
    /// Use -- to separate jail-ai options from agent arguments
    /// Example: jail-ai gemini --gemini-dir -- --model gemini-pro "query"
    Gemini {
        #[command(flatten)]
        common: AgentCommandOptions,

        /// Additional arguments to pass to gemini (after --)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Quick start Codex CLI in a jail for current directory
    /// Use -- to separate jail-ai options from agent arguments
    /// Example: jail-ai codex --codex-dir -- generate "create API"
    Codex {
        #[command(flatten)]
        common: AgentCommandOptions,

        /// Additional arguments to pass to codex (after --)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// List all jails
    List {
        /// Show only jails for current directory
        #[arg(short, long)]
        current: bool,

        /// Backend type (only 'podman' is supported, kept for compatibility)
        #[arg(short, long)]
        backend: Option<String>,
    },

    /// Stop and remove all jail-ai containers
    CleanAll {
        /// Backend type (only 'podman' is supported, kept for compatibility)
        #[arg(short, long)]
        backend: Option<String>,

        /// Force removal without confirmation
        #[arg(short, long)]
        force: bool,

        /// Remove associated volumes (persistent data)
        #[arg(short, long)]
        volume: bool,
    },

    /// Upgrade jail by recreating it with the latest image
    Upgrade {
        /// Name of the jail (auto-detected from current directory if not provided)
        name: Option<String>,

        /// Base image to upgrade to (e.g., localhost/jail-ai-env:latest, alpine:latest)
        #[arg(short, long)]
        image: Option<String>,

        /// Force upgrade without confirmation
        #[arg(short, long)]
        force: bool,

        /// Upgrade all jails
        #[arg(long)]
        all: bool,
    },
}

impl Commands {
    pub fn parse_backend(backend: &str) -> Result<crate::config::BackendType, String> {
        match backend.to_lowercase().as_str() {
            "podman" | "pod" => Ok(crate::config::BackendType::Podman),
            _ => Err(format!(
                "Invalid backend '{backend}'. Only 'podman' is supported"
            )),
        }
    }

    pub fn parse_mount(mount_str: &str) -> Result<crate::config::BindMount, String> {
        let parts: Vec<&str> = mount_str.split(':').collect();
        if parts.len() < 2 {
            return Err(format!(
                "Invalid mount format '{mount_str}'. Expected: source:target[:ro]"
            ));
        }

        let readonly = parts.get(2).is_some_and(|&s| s == "ro");
        let source = PathBuf::from(parts[0]);
        let target = PathBuf::from(parts[1]);

        // Validate mount source is safe
        crate::validate_mount_source(&source).map_err(|e| e.to_string())?;

        Ok(crate::config::BindMount {
            source,
            target,
            readonly,
        })
    }

    pub fn parse_env(env_str: &str) -> Result<(String, String), String> {
        let parts: Vec<&str> = env_str.splitn(2, '=').collect();
        if parts.len() != 2 {
            return Err(format!(
                "Invalid environment variable format '{env_str}'. Expected: KEY=VALUE"
            ));
        }

        Ok((parts[0].to_string(), parts[1].to_string()))
    }

    /// Sanitize a jail name to match podman requirements
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
            Commands::parse_backend("pod"),
            Ok(crate::config::BackendType::Podman)
        ));
        assert!(Commands::parse_backend("invalid").is_err());
    }

    #[test]
    fn test_parse_mount() {
        // Test with a path that exists
        let mount = Commands::parse_mount("/tmp:/dst:ro").unwrap();
        assert_eq!(mount.source, PathBuf::from("/tmp"));
        assert_eq!(mount.target, PathBuf::from("/dst"));
        assert!(mount.readonly);

        let mount = Commands::parse_mount("/tmp:/dst").unwrap();
        assert!(!mount.readonly);

        assert!(Commands::parse_mount("invalid").is_err());

        // Test unsafe mount validation
        assert!(Commands::parse_mount("/:/dst").is_err());
        // Test with actual home directory path (should fail)
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/agent".to_string());
        assert!(Commands::parse_mount(&format!("{}:/dst", home)).is_err());
        // Test with home subdirectory (should pass if directory exists)
        // Use .config which should exist in most cases
        let home_config = format!("{}/.config:/dst", home);
        if std::path::Path::new(&home_config.split(':').next().unwrap()).exists() {
            assert!(Commands::parse_mount(&home_config).is_ok());
        }
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

    #[test]
    fn test_auth_parameter_parsing() {
        // Test that the --auth parameter is properly parsed
        let args = vec!["jail-ai", "codex", "--auth", "sk-test123", "--codex-dir"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Some(Commands::Codex { common, .. }) => {
                assert_eq!(common.auth, Some("sk-test123".to_string()));
                assert!(common.codex_dir);
            }
            _ => panic!("Expected Codex command"),
        }
    }

    #[test]
    fn test_auth_parameter_optional() {
        // Test that the --auth parameter is optional
        let args = vec!["jail-ai", "codex", "--codex-dir"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Some(Commands::Codex { common, .. }) => {
                assert_eq!(common.auth, None);
                assert!(common.codex_dir);
            }
            _ => panic!("Expected Codex command"),
        }
    }
}
