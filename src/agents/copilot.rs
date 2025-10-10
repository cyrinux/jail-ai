//! GitHub Copilot CLI agent configuration
//!
//! GitHub Copilot CLI is GitHub's AI pair programmer for the terminal.
//! https://docs.github.com/en/copilot/github-copilot-in-the-cli

/// Command name to execute the agent
pub const COMMAND_NAME: &str = "copilot";

/// Normalized name for jail naming and images
pub const NORMALIZED_NAME: &str = "copilot";

/// Display name for UI
pub const DISPLAY_NAME: &str = "GitHub Copilot";

/// Auto-mount credentials (minimal auth)
pub const HAS_AUTO_CREDENTIALS: bool = false;

/// Config directory paths: (host_path, container_path)
pub const CONFIG_DIR_PATHS: &[(&str, &str)] =
    &[(".config/.copilot", "/home/agent/.config/.copilot")];
