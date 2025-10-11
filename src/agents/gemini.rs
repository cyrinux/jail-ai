//! Gemini CLI agent configuration
//!
//! Gemini CLI is Google's AI terminal assistant.
//! https://ai.google.dev

/// Command name to execute the agent
pub const COMMAND_NAME: &str = "gemini";

/// Normalized name for jail naming and images
pub const NORMALIZED_NAME: &str = "gemini";

/// Display name for UI
pub const DISPLAY_NAME: &str = "Gemini";

/// Auto-mount credentials (minimal auth)
pub const HAS_AUTO_CREDENTIALS: bool = false;

/// Config directory paths: (host_path, container_path)
pub const CONFIG_DIR_PATHS: &[(&str, &str)] = &[(".gemini", "/home/agent/.gemini")];

/// Supports OAuth authentication workflow with network=host
pub const SUPPORTS_AUTH_WORKFLOW: bool = false;
