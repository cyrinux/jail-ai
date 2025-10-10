//! Codex CLI agent configuration
//!
//! Codex CLI is OpenAI's Codex CLI for code generation.
//! Note: Supports API key authentication for programmatic access.

/// Command name to execute the agent
pub const COMMAND_NAME: &str = "codex";

/// Normalized name for jail naming and images
pub const NORMALIZED_NAME: &str = "codex";

/// Display name for UI
pub const DISPLAY_NAME: &str = "Codex";

/// Auto-mount credentials (minimal auth)
pub const HAS_AUTO_CREDENTIALS: bool = false;

/// Supports API key authentication
pub const SUPPORTS_API_KEY_AUTH: bool = true;

/// Config directory paths: (host_path, container_path)
pub const CONFIG_DIR_PATHS: &[(&str, &str)] = &[(".codex", "/home/agent/.codex")];
