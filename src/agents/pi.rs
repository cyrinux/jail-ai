//! Pi agent configuration
//!
//! Pi is a lightweight AI coding assistant CLI.
//! https://github.com/mariozechner/pi-coding-agent

/// Command name to execute the agent
pub const COMMAND_NAME: &str = "pi";

/// Normalized name for jail naming and images
pub const NORMALIZED_NAME: &str = "pi";

/// Display name for UI
pub const DISPLAY_NAME: &str = "Pi";

/// Auto-mount credentials (minimal auth) - mount entire ~/.pi dir automatically
pub const HAS_AUTO_CREDENTIALS: bool = true;

/// Config directory paths: (host_path, container_path)
pub const CONFIG_DIR_PATHS: &[(&str, &str)] = &[(".pi", "/home/agent/.pi")];

/// Supports OAuth authentication workflow with network=host
pub const SUPPORTS_AUTH_WORKFLOW: bool = false;

/// Path to the auth credential file/directory to check for first run
/// Relative to user's home directory
pub const AUTH_CREDENTIAL_PATH: &str = ".pi";
