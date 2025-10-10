//! Jules CLI agent configuration
//!
//! Jules is Google's AI coding assistant CLI.
//! https://jules.google.com

/// Command name to execute the agent
pub const COMMAND_NAME: &str = "jules";

/// Normalized name for jail naming and images
pub const NORMALIZED_NAME: &str = "jules";

/// Display name for UI
pub const DISPLAY_NAME: &str = "Jules";

/// Auto-mount credentials (minimal auth)
pub const HAS_AUTO_CREDENTIALS: bool = false;

/// Supports API key authentication
pub const SUPPORTS_API_KEY_AUTH: bool = false;

/// Config directory paths: (host_path, container_path)
pub const CONFIG_DIR_PATHS: &[(&str, &str)] = &[(".config/jules", "/home/agent/.config/jules")];
