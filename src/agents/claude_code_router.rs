//! Claude Code Router agent configuration
//!
//! Claude Code Router is a powerful tool to route Claude Code requests to different models.
//! It wraps Claude Code and allows routing to various providers (OpenRouter, DeepSeek, Ollama, etc.)
//! https://github.com/musistudio/claude-code-router
//!
//! IMPORTANT: Claude Code Router requires starting a server first with "ccr start"
//! before running the main command "ccr code". This is handled automatically.

/// Command name to execute the agent
pub const COMMAND_NAME: &str = "ccr";

/// Normalized name for jail naming and images
pub const NORMALIZED_NAME: &str = "claude-code-router";

/// Display name for UI
pub const DISPLAY_NAME: &str = "Claude Code Router";

/// Auto-mount credentials (minimal auth)
/// Claude Code Router requires both .claude and .claude-code-router directories
pub const HAS_AUTO_CREDENTIALS: bool = false;

/// Config directory paths: (host_path, container_path)
/// Requires both .claude and .claude-code-router directories
pub const CONFIG_DIR_PATHS: &[(&str, &str)] = &[
    (".claude", "/home/agent/.claude"),
    (".claude-code-router", "/home/agent/.claude-code-router"),
];

/// Supports OAuth authentication workflow with network=host
pub const SUPPORTS_AUTH_WORKFLOW: bool = false;

/// Path to the auth credential file/directory to check for first run
/// Relative to user's home directory
pub const AUTH_CREDENTIAL_PATH: &str = ".claude-code-router";

/// Requires a server to be started before executing commands
/// This agent needs "ccr start" to be run first, then "ccr code" can be executed
pub const REQUIRES_SERVER_START: bool = true;

/// Server start command (executed in the background)
pub const SERVER_START_COMMAND: &str = "start";

/// Main command to execute after server is started
pub const MAIN_COMMAND: &str = "code";
