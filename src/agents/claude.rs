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

/// Supports API key authentication
pub const SUPPORTS_API_KEY_AUTH: bool = false;

/// Config directory paths: (host_path, container_path)
pub const CONFIG_DIR_PATHS: &[(&str, &str)] = &[(".claude", "/home/agent/.claude")];
