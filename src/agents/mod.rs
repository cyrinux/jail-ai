//! Agent registry for managing AI agent configurations.
//!
//! This module provides a centralized way to manage different AI coding agents
//! (Claude, Copilot, Cursor, Gemini, Codex, Jules) with type-safe configuration.
//!
//! # Architecture
//!
//! Each agent has its own module with specific configuration:
//! - Command name (how to execute it)
//! - Config directory paths
//! - Authentication methods
//! - Display metadata
//!
//! # Adding a New Agent
//!
//! 1. Create a new file in `src/agents/` (e.g., `newagent.rs`)
//! 2. Define the agent's configuration using the `AgentConfig` struct
//! 3. Add the agent to the `Agent` enum in this file
//! 4. Implement the enum methods for the new variant
//! 5. Add the module declaration below

mod claude;
mod codex;
mod copilot;
mod cursor;
mod gemini;
mod jules;

use std::fmt;

/// Supported AI agents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Agent {
    Claude,
    Copilot,
    Cursor,
    Gemini,
    Codex,
    Jules,
}

impl Agent {
    /// Parse agent from string (used in CLI parsing and jail name detection)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "claude" => Some(Self::Claude),
            "copilot" => Some(Self::Copilot),
            "cursor" | "cursor-agent" => Some(Self::Cursor),
            "gemini" => Some(Self::Gemini),
            "codex" => Some(Self::Codex),
            "jules" => Some(Self::Jules),
            _ => None,
        }
    }

    /// Get the command name used to execute the agent
    pub fn command_name(&self) -> &'static str {
        match self {
            Self::Claude => claude::COMMAND_NAME,
            Self::Copilot => copilot::COMMAND_NAME,
            Self::Cursor => cursor::COMMAND_NAME,
            Self::Gemini => gemini::COMMAND_NAME,
            Self::Codex => codex::COMMAND_NAME,
            Self::Jules => jules::COMMAND_NAME,
        }
    }

    /// Get the normalized agent name for jail naming and image building
    pub fn normalized_name(&self) -> &'static str {
        match self {
            Self::Claude => claude::NORMALIZED_NAME,
            Self::Copilot => copilot::NORMALIZED_NAME,
            Self::Cursor => cursor::NORMALIZED_NAME,
            Self::Gemini => gemini::NORMALIZED_NAME,
            Self::Codex => codex::NORMALIZED_NAME,
            Self::Jules => jules::NORMALIZED_NAME,
        }
    }

    /// Get the display name for UI messages
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Claude => claude::DISPLAY_NAME,
            Self::Copilot => copilot::DISPLAY_NAME,
            Self::Cursor => cursor::DISPLAY_NAME,
            Self::Gemini => gemini::DISPLAY_NAME,
            Self::Codex => codex::DISPLAY_NAME,
            Self::Jules => jules::DISPLAY_NAME,
        }
    }

    /// Get the agent layer name for container images
    pub fn layer_name(&self) -> String {
        format!("agent-{}", self.normalized_name())
    }

    /// Check if this agent has auto-mounted credentials (minimal auth)
    pub fn has_auto_credentials(&self) -> bool {
        match self {
            Self::Claude => claude::HAS_AUTO_CREDENTIALS,
            Self::Copilot => copilot::HAS_AUTO_CREDENTIALS,
            Self::Cursor => cursor::HAS_AUTO_CREDENTIALS,
            Self::Gemini => gemini::HAS_AUTO_CREDENTIALS,
            Self::Codex => codex::HAS_AUTO_CREDENTIALS,
            Self::Jules => jules::HAS_AUTO_CREDENTIALS,
        }
    }

    /// Get all config directory paths for agents
    /// Returns (host_path, container_path) tuples
    pub fn config_dir_paths(&self) -> Vec<(&'static str, &'static str)> {
        match self {
            Self::Claude => claude::CONFIG_DIR_PATHS.to_vec(),
            Self::Copilot => copilot::CONFIG_DIR_PATHS.to_vec(),
            Self::Cursor => cursor::CONFIG_DIR_PATHS.to_vec(),
            Self::Gemini => gemini::CONFIG_DIR_PATHS.to_vec(),
            Self::Codex => codex::CONFIG_DIR_PATHS.to_vec(),
            Self::Jules => jules::CONFIG_DIR_PATHS.to_vec(),
        }
    }

    /// Check if this agent supports the OAuth authentication workflow with network=host
    pub fn supports_auth_workflow(&self) -> bool {
        match self {
            Self::Claude => claude::SUPPORTS_AUTH_WORKFLOW,
            Self::Copilot => copilot::SUPPORTS_AUTH_WORKFLOW,
            Self::Cursor => cursor::SUPPORTS_AUTH_WORKFLOW,
            Self::Gemini => gemini::SUPPORTS_AUTH_WORKFLOW,
            Self::Codex => codex::SUPPORTS_AUTH_WORKFLOW,
            Self::Jules => jules::SUPPORTS_AUTH_WORKFLOW,
        }
    }
}

impl fmt::Display for Agent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Extract agent from jail name
/// Jail name format: jail-{project}-{hash}-{agent}
pub fn extract_agent_from_jail_name(jail_name: &str) -> Option<Agent> {
    if !jail_name.starts_with("jail-") {
        return None;
    }

    // The format is: jail-{project}-{hash}-{agent}
    // The hash is always 8 characters (hexadecimal)
    let parts: Vec<&str> = jail_name.split('-').collect();

    // Look for a part that is exactly 8 characters and all hex digits (the hash)
    for (i, part) in parts.iter().enumerate() {
        if part.len() == 8 && part.chars().all(|c| c.is_ascii_hexdigit()) {
            // Found the hash at index i, agent starts after it
            if i + 1 < parts.len() {
                // Join remaining parts in case agent name has hyphens
                let agent_str = parts[i + 1..].join("-");
                return Agent::from_str(&agent_str);
            }
        }
    }

    // Fallback: check last part
    parts
        .last()
        .and_then(|&agent_str| Agent::from_str(agent_str))
}

/// Get a friendly display name for an agent extracted from a jail name
/// Returns a static string to avoid allocations and memory leaks
pub fn get_agent_display_name(jail_name: &str) -> &'static str {
    extract_agent_from_jail_name(jail_name)
        .map(|a| a.display_name())
        .unwrap_or("unknown")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_from_str() {
        assert_eq!(Agent::from_str("claude"), Some(Agent::Claude));
        assert_eq!(Agent::from_str("copilot"), Some(Agent::Copilot));
        assert_eq!(Agent::from_str("cursor"), Some(Agent::Cursor));
        assert_eq!(Agent::from_str("cursor-agent"), Some(Agent::Cursor));
        assert_eq!(Agent::from_str("gemini"), Some(Agent::Gemini));
        assert_eq!(Agent::from_str("codex"), Some(Agent::Codex));
        assert_eq!(Agent::from_str("jules"), Some(Agent::Jules));
        assert_eq!(Agent::from_str("unknown"), None);
        assert_eq!(Agent::from_str("CLAUDE"), Some(Agent::Claude)); // case-insensitive
    }

    #[test]
    fn test_agent_command_name() {
        assert_eq!(Agent::Claude.command_name(), "claude");
        assert_eq!(Agent::Copilot.command_name(), "copilot");
        assert_eq!(Agent::Cursor.command_name(), "cursor-agent");
        assert_eq!(Agent::Gemini.command_name(), "gemini");
        assert_eq!(Agent::Codex.command_name(), "codex");
        assert_eq!(Agent::Jules.command_name(), "jules");
    }

    #[test]
    fn test_agent_normalized_name() {
        assert_eq!(Agent::Claude.normalized_name(), "claude");
        assert_eq!(Agent::Cursor.normalized_name(), "cursor");
    }

    #[test]
    fn test_extract_agent_from_jail_name() {
        assert_eq!(
            extract_agent_from_jail_name("jail-myproject-abc12345-claude"),
            Some(Agent::Claude)
        );
        assert_eq!(
            extract_agent_from_jail_name("jail-test-def67890-cursor"),
            Some(Agent::Cursor)
        );
        assert_eq!(
            extract_agent_from_jail_name("jail-foo-12ab34cd-copilot"),
            Some(Agent::Copilot)
        );
        assert_eq!(extract_agent_from_jail_name("not-a-jail"), None);
        assert_eq!(extract_agent_from_jail_name("jail-invalid"), None);
    }

    #[test]
    fn test_agent_has_auto_credentials() {
        assert!(Agent::Claude.has_auto_credentials());
        assert!(!Agent::Copilot.has_auto_credentials());
        assert!(!Agent::Cursor.has_auto_credentials());
        assert!(!Agent::Gemini.has_auto_credentials());
        assert!(!Agent::Codex.has_auto_credentials());
        assert!(!Agent::Jules.has_auto_credentials());
    }

    #[test]
    fn test_agent_layer_name() {
        assert_eq!(Agent::Claude.layer_name(), "agent-claude");
        assert_eq!(Agent::Cursor.layer_name(), "agent-cursor");
    }
}
