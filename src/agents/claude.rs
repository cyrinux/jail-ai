//! Claude AI agent configuration
//!
//! Claude Code is Anthropic's official CLI for Claude AI.
//! https://claude.ai/code

/// Command name to execute the agent
pub const COMMAND_NAME: &str = "claude";

/// Normalized name for jail naming and images
pub const NORMALIZED_NAME: &str = "claude";

/// Display name for UI
pub const DISPLAY_NAME: &str = "Claude";

/// Auto-mount credentials (minimal auth)
pub const HAS_AUTO_CREDENTIALS: bool = true;

/// Config directory paths: (host_path, container_path)
pub const CONFIG_DIR_PATHS: &[(&str, &str)] = &[(".claude", "/home/agent/.claude")];

/// Supports OAuth authentication workflow with network=host
pub const SUPPORTS_AUTH_WORKFLOW: bool = false;

/// Path to the auth credential file/directory to check for first run
/// Relative to user's home directory
pub const AUTH_CREDENTIAL_PATH: &str = ".claude/.credentials.json";
