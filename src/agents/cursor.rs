//! Cursor Agent configuration
//!
//! Cursor Agent is Cursor's terminal AI assistant.
//! https://cursor.sh

/// Command name to execute the agent (uses cursor-agent command)
pub const COMMAND_NAME: &str = "cursor-agent";

/// Normalized name for jail naming and images
pub const NORMALIZED_NAME: &str = "cursor";

/// Display name for UI
pub const DISPLAY_NAME: &str = "Cursor";

/// Auto-mount credentials (minimal auth)
pub const HAS_AUTO_CREDENTIALS: bool = false;

/// Config directory paths: (host_path, container_path)
/// Cursor uses multiple config directories
pub const CONFIG_DIR_PATHS: &[(&str, &str)] = &[
    (".cursor", "/home/agent/.cursor"),
    (".config/cursor", "/home/agent/.config/cursor"),
];
