//! Agent registry for managing AI agent configurations.
//!
//! This module provides a centralized way to manage different AI coding agents
//! (Claude, Copilot, Cursor, Gemini, Codex, Jules, Claude Code Router) with type-safe configuration.
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
mod claude_code_router;
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
    ClaudeCodeRouter,
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
            "claude-code-router" | "ccr" => Some(Self::ClaudeCodeRouter),
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
            Self::ClaudeCodeRouter => claude_code_router::COMMAND_NAME,
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
            Self::ClaudeCodeRouter => claude_code_router::NORMALIZED_NAME,
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
            Self::ClaudeCodeRouter => claude_code_router::DISPLAY_NAME,
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
            Self::ClaudeCodeRouter => claude_code_router::HAS_AUTO_CREDENTIALS,
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
            Self::ClaudeCodeRouter => claude_code_router::CONFIG_DIR_PATHS.to_vec(),
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
            Self::ClaudeCodeRouter => claude_code_router::SUPPORTS_AUTH_WORKFLOW,
            Self::Copilot => copilot::SUPPORTS_AUTH_WORKFLOW,
            Self::Cursor => cursor::SUPPORTS_AUTH_WORKFLOW,
            Self::Gemini => gemini::SUPPORTS_AUTH_WORKFLOW,
            Self::Codex => codex::SUPPORTS_AUTH_WORKFLOW,
            Self::Jules => jules::SUPPORTS_AUTH_WORKFLOW,
        }
    }

    /// Get the auth credential path to check for first run
    /// Returns relative path from user's home directory
    pub fn auth_credential_path(&self) -> &'static str {
        match self {
            Self::Claude => claude::AUTH_CREDENTIAL_PATH,
            Self::ClaudeCodeRouter => claude_code_router::AUTH_CREDENTIAL_PATH,
            Self::Copilot => copilot::AUTH_CREDENTIAL_PATH,
            Self::Cursor => cursor::AUTH_CREDENTIAL_PATH,
            Self::Gemini => gemini::AUTH_CREDENTIAL_PATH,
            Self::Codex => codex::AUTH_CREDENTIAL_PATH,
            Self::Jules => jules::AUTH_CREDENTIAL_PATH,
        }
    }

    /// Check if authentication credentials exist and are not empty
    /// Returns true if credentials are missing or empty (first run)
    pub fn needs_auth(&self, home_dir: &std::path::Path) -> bool {
        let cred_path = home_dir.join(self.auth_credential_path());

        // Check if path exists
        if !cred_path.exists() {
            return true;
        }

        // Check if it's a file and is empty
        if cred_path.is_file() {
            if let Ok(metadata) = std::fs::metadata(&cred_path) {
                if metadata.len() == 0 {
                    return true;
                }
            }
        }

        // Check if it's a directory and is empty
        if cred_path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&cred_path) {
                if entries.count() == 0 {
                    return true;
                }
            }
        }

        false
    }

    /// Get the specific config flag name for this agent (e.g., "claude-dir", "copilot-dir")
    pub fn config_flag_name(&self) -> &'static str {
        match self {
            Self::Claude => "claude-dir",
            Self::ClaudeCodeRouter => "claude-code-router-dir",
            Self::Copilot => "copilot-dir",
            Self::Cursor => "cursor-dir",
            Self::Gemini => "gemini-dir",
            Self::Codex => "codex-dir",
            Self::Jules => "jules-dir",
        }
    }

    /// Check if this agent requires a server to be started before executing the main command
    pub fn requires_server_start(&self) -> bool {
        match self {
            Self::ClaudeCodeRouter => claude_code_router::REQUIRES_SERVER_START,
            _ => false,
        }
    }

    /// Get the server start command for this agent
    pub fn server_start_command(&self) -> Option<&'static str> {
        match self {
            Self::ClaudeCodeRouter => Some(claude_code_router::SERVER_START_COMMAND),
            _ => None,
        }
    }

    /// Get the main command to execute after the server is started
    pub fn main_command(&self) -> Option<&'static str> {
        match self {
            Self::ClaudeCodeRouter => Some(claude_code_router::MAIN_COMMAND),
            _ => None,
        }
    }

    /// Validate that only compatible agent config flags are being used
    /// Returns an error message if incompatible flags are detected
    pub fn validate_config_flags(&self, flags: &AgentConfigFlags) -> Result<(), String> {
        // Build a list of all specified flags
        let mut specified_flags = Vec::new();

        if flags.claude_dir {
            specified_flags.push(("claude-dir", Agent::Claude));
        }
        if flags.claude_code_router_dir {
            specified_flags.push(("claude-code-router-dir", Agent::ClaudeCodeRouter));
        }
        if flags.copilot_dir {
            specified_flags.push(("copilot-dir", Agent::Copilot));
        }
        if flags.cursor_dir {
            specified_flags.push(("cursor-dir", Agent::Cursor));
        }
        if flags.gemini_dir {
            specified_flags.push(("gemini-dir", Agent::Gemini));
        }
        if flags.codex_dir {
            specified_flags.push(("codex-dir", Agent::Codex));
        }
        if flags.jules_dir {
            specified_flags.push(("jules-dir", Agent::Jules));
        }

        // If --agent-configs is specified, allow all flags
        if flags.agent_configs {
            return Ok(());
        }

        // Check if any incompatible flags are specified
        let incompatible_flags: Vec<&str> = specified_flags
            .iter()
            .filter(|(_, agent)| agent != self)
            .map(|(flag_name, _)| *flag_name)
            .collect();

        if !incompatible_flags.is_empty() {
            let flags_list = incompatible_flags.join(", ");
            return Err(format!(
                "Cannot use --{} with {} agent. Use --{} instead, or use --agent-configs to mount all agent directories.",
                flags_list,
                self.display_name(),
                self.config_flag_name()
            ));
        }

        Ok(())
    }
}

/// Agent configuration flags structure for validation
pub struct AgentConfigFlags {
    pub claude_dir: bool,
    pub claude_code_router_dir: bool,
    pub copilot_dir: bool,
    pub cursor_dir: bool,
    pub gemini_dir: bool,
    pub codex_dir: bool,
    pub jules_dir: bool,
    pub agent_configs: bool,
}

impl fmt::Display for Agent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Extract agent from jail name
/// Jail name format: jail__{project}__{hash}__{agent}
pub fn extract_agent_from_jail_name(jail_name: &str) -> Option<Agent> {
    if !jail_name.starts_with("jail__") {
        return None;
    }

    // The format is: jail__{project}__{hash}__{agent}
    // Simply get the last segment after __
    jail_name.rsplit("__").next().and_then(Agent::from_str)
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
        assert_eq!(
            Agent::from_str("claude-code-router"),
            Some(Agent::ClaudeCodeRouter)
        );
        assert_eq!(Agent::from_str("ccr"), Some(Agent::ClaudeCodeRouter));
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
        assert_eq!(Agent::ClaudeCodeRouter.command_name(), "ccr");
        assert_eq!(Agent::Copilot.command_name(), "copilot");
        assert_eq!(Agent::Cursor.command_name(), "cursor-agent");
        assert_eq!(Agent::Gemini.command_name(), "gemini");
        assert_eq!(Agent::Codex.command_name(), "codex");
        assert_eq!(Agent::Jules.command_name(), "jules");
    }

    #[test]
    fn test_agent_normalized_name() {
        assert_eq!(Agent::Claude.normalized_name(), "claude");
        assert_eq!(
            Agent::ClaudeCodeRouter.normalized_name(),
            "claude-code-router"
        );
        assert_eq!(Agent::Cursor.normalized_name(), "cursor");
    }

    #[test]
    fn test_extract_agent_from_jail_name() {
        assert_eq!(
            extract_agent_from_jail_name("jail__myproject__abc12345__claude"),
            Some(Agent::Claude)
        );
        assert_eq!(
            extract_agent_from_jail_name("jail__test__def67890__cursor"),
            Some(Agent::Cursor)
        );
        assert_eq!(
            extract_agent_from_jail_name("jail__foo__12ab34cd__copilot"),
            Some(Agent::Copilot)
        );
        assert_eq!(extract_agent_from_jail_name("not-a-jail"), None);
        assert_eq!(extract_agent_from_jail_name("jail__invalid"), None);
    }

    #[test]
    fn test_agent_has_auto_credentials() {
        assert!(Agent::Claude.has_auto_credentials());
        assert!(!Agent::ClaudeCodeRouter.has_auto_credentials());
        assert!(!Agent::Copilot.has_auto_credentials());
        assert!(!Agent::Cursor.has_auto_credentials());
        assert!(!Agent::Gemini.has_auto_credentials());
        assert!(!Agent::Codex.has_auto_credentials());
        assert!(!Agent::Jules.has_auto_credentials());
    }

    #[test]
    fn test_agent_layer_name() {
        assert_eq!(Agent::Claude.layer_name(), "agent-claude");
        assert_eq!(
            Agent::ClaudeCodeRouter.layer_name(),
            "agent-claude-code-router"
        );
        assert_eq!(Agent::Cursor.layer_name(), "agent-cursor");
    }

    #[test]
    fn test_agent_auth_credential_path() {
        assert_eq!(
            Agent::Claude.auth_credential_path(),
            ".claude/.credentials.json"
        );
        assert_eq!(Agent::Copilot.auth_credential_path(), ".config/.copilot");
        assert_eq!(Agent::Cursor.auth_credential_path(), ".cursor");
        assert_eq!(Agent::Gemini.auth_credential_path(), ".gemini");
        assert_eq!(Agent::Codex.auth_credential_path(), ".codex");
        assert_eq!(Agent::Jules.auth_credential_path(), ".config/jules");
    }

    #[test]
    fn test_agent_needs_auth() {
        use std::fs;
        use tempfile::TempDir;

        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let home_path = temp_dir.path();

        // Test 1: Missing credentials directory - should need auth
        assert!(Agent::Codex.needs_auth(home_path));

        // Test 2: Empty credentials directory - should need auth
        let codex_dir = home_path.join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        assert!(Agent::Codex.needs_auth(home_path));

        // Test 3: Non-empty credentials directory - should not need auth
        let cred_file = codex_dir.join("credentials.json");
        fs::write(cred_file, "{}").unwrap();
        assert!(!Agent::Codex.needs_auth(home_path));

        // Test 4: Empty file - should need auth
        let claude_creds = home_path.join(".claude/.credentials.json");
        fs::create_dir_all(claude_creds.parent().unwrap()).unwrap();
        fs::write(&claude_creds, "").unwrap();
        assert!(Agent::Claude.needs_auth(home_path));

        // Test 5: Non-empty file - should not need auth
        fs::write(&claude_creds, r#"{"api_key": "test"}"#).unwrap();
        assert!(!Agent::Claude.needs_auth(home_path));
    }

    #[test]
    fn test_agent_config_flag_name() {
        assert_eq!(Agent::Claude.config_flag_name(), "claude-dir");
        assert_eq!(
            Agent::ClaudeCodeRouter.config_flag_name(),
            "claude-code-router-dir"
        );
        assert_eq!(Agent::Copilot.config_flag_name(), "copilot-dir");
        assert_eq!(Agent::Cursor.config_flag_name(), "cursor-dir");
        assert_eq!(Agent::Gemini.config_flag_name(), "gemini-dir");
        assert_eq!(Agent::Codex.config_flag_name(), "codex-dir");
        assert_eq!(Agent::Jules.config_flag_name(), "jules-dir");
    }

    #[test]
    fn test_validate_config_flags_matching_agent() {
        // Test that matching flags pass validation
        let flags = AgentConfigFlags {
            claude_dir: true,
            claude_code_router_dir: false,
            copilot_dir: false,
            cursor_dir: false,
            gemini_dir: false,
            codex_dir: false,
            jules_dir: false,
            agent_configs: false,
        };
        assert!(Agent::Claude.validate_config_flags(&flags).is_ok());

        let flags = AgentConfigFlags {
            claude_dir: false,
            claude_code_router_dir: false,
            copilot_dir: true,
            cursor_dir: false,
            gemini_dir: false,
            codex_dir: false,
            jules_dir: false,
            agent_configs: false,
        };
        assert!(Agent::Copilot.validate_config_flags(&flags).is_ok());
    }

    #[test]
    fn test_validate_config_flags_mismatched_agent() {
        // Test that mismatched flags fail validation
        let flags = AgentConfigFlags {
            claude_dir: false,
            claude_code_router_dir: false,
            copilot_dir: false,
            cursor_dir: false,
            gemini_dir: true, // Wrong flag for Cursor agent
            codex_dir: false,
            jules_dir: false,
            agent_configs: false,
        };
        let result = Agent::Cursor.validate_config_flags(&flags);
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("gemini-dir"));
        assert!(error_msg.contains("Cursor agent"));
        assert!(error_msg.contains("cursor-dir"));
    }

    #[test]
    fn test_validate_config_flags_multiple_wrong_flags() {
        // Test that multiple mismatched flags are all reported
        let flags = AgentConfigFlags {
            claude_dir: true,
            claude_code_router_dir: false,
            copilot_dir: true,
            cursor_dir: false,
            gemini_dir: true,
            codex_dir: false,
            jules_dir: false,
            agent_configs: false,
        };
        let result = Agent::Cursor.validate_config_flags(&flags);
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("claude-dir"));
        assert!(error_msg.contains("copilot-dir"));
        assert!(error_msg.contains("gemini-dir"));
    }

    #[test]
    fn test_validate_config_flags_with_agent_configs() {
        // Test that --agent-configs allows all flags
        let flags = AgentConfigFlags {
            claude_dir: true,
            claude_code_router_dir: true,
            copilot_dir: true,
            cursor_dir: true,
            gemini_dir: true,
            codex_dir: true,
            jules_dir: true,
            agent_configs: true, // This should allow everything
        };
        assert!(Agent::Claude.validate_config_flags(&flags).is_ok());
        assert!(Agent::Copilot.validate_config_flags(&flags).is_ok());
        assert!(Agent::Cursor.validate_config_flags(&flags).is_ok());
        assert!(Agent::Gemini.validate_config_flags(&flags).is_ok());
        assert!(Agent::Codex.validate_config_flags(&flags).is_ok());
        assert!(Agent::Jules.validate_config_flags(&flags).is_ok());
    }

    #[test]
    fn test_validate_config_flags_no_flags() {
        // Test that having no flags always passes
        let flags = AgentConfigFlags {
            claude_dir: false,
            claude_code_router_dir: false,
            copilot_dir: false,
            cursor_dir: false,
            gemini_dir: false,
            codex_dir: false,
            jules_dir: false,
            agent_configs: false,
        };
        assert!(Agent::Claude.validate_config_flags(&flags).is_ok());
        assert!(Agent::Copilot.validate_config_flags(&flags).is_ok());
        assert!(Agent::Cursor.validate_config_flags(&flags).is_ok());
    }
}
