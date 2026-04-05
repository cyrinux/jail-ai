mod claude;
mod claude_code_router;
mod coderabbit;
mod codex;
mod copilot;
mod cursor;
mod gemini;
mod jules;
mod opencode;
mod pi;

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Agent {
    Claude,
    ClaudeCodeRouter,
    CodeRabbit,
    Copilot,
    Cursor,
    Gemini,
    Codex,
    Jules,
    OpenCode,
    Pi,
}

pub const ALL_AGENTS: &[Agent] = &[
    Agent::Claude,
    Agent::ClaudeCodeRouter,
    Agent::CodeRabbit,
    Agent::Copilot,
    Agent::Cursor,
    Agent::Gemini,
    Agent::Codex,
    Agent::Jules,
    Agent::OpenCode,
    Agent::Pi,
];

macro_rules! agent_dispatch {
    ($self:expr, $field:ident) => {
        match $self {
            Self::Claude => claude::$field,
            Self::ClaudeCodeRouter => claude_code_router::$field,
            Self::CodeRabbit => coderabbit::$field,
            Self::Copilot => copilot::$field,
            Self::Cursor => cursor::$field,
            Self::Gemini => gemini::$field,
            Self::Codex => codex::$field,
            Self::Jules => jules::$field,
            Self::OpenCode => opencode::$field,
            Self::Pi => pi::$field,
        }
    };
}

impl Agent {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "claude" => Some(Self::Claude),
            "claude-code-router" | "ccr" => Some(Self::ClaudeCodeRouter),
            "coderabbit" | "code-rabbit" => Some(Self::CodeRabbit),
            "copilot" => Some(Self::Copilot),
            "cursor" | "cursor-agent" => Some(Self::Cursor),
            "gemini" => Some(Self::Gemini),
            "codex" => Some(Self::Codex),
            "jules" => Some(Self::Jules),
            "opencode" => Some(Self::OpenCode),
            "pi" => Some(Self::Pi),
            _ => None,
        }
    }

    pub fn command_name(&self) -> &'static str {
        agent_dispatch!(self, COMMAND_NAME)
    }

    pub fn normalized_name(&self) -> &'static str {
        agent_dispatch!(self, NORMALIZED_NAME)
    }

    pub fn display_name(&self) -> &'static str {
        agent_dispatch!(self, DISPLAY_NAME)
    }

    pub fn layer_name(&self) -> String {
        format!("agent-{}", self.normalized_name())
    }

    pub fn has_auto_credentials(&self) -> bool {
        agent_dispatch!(self, HAS_AUTO_CREDENTIALS)
    }

    pub fn config_dir_paths(&self) -> Vec<(&'static str, &'static str)> {
        agent_dispatch!(self, CONFIG_DIR_PATHS).to_vec()
    }

    pub fn supports_auth_workflow(&self) -> bool {
        agent_dispatch!(self, SUPPORTS_AUTH_WORKFLOW)
    }

    pub fn auth_credential_path(&self) -> &'static str {
        agent_dispatch!(self, AUTH_CREDENTIAL_PATH)
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Claude => "🤖",
            Self::ClaudeCodeRouter => "🔀",
            Self::CodeRabbit => "🐰",
            Self::Copilot => "🦾",
            Self::Cursor => "➡️",
            Self::Gemini => "🔮",
            Self::Codex => "💻",
            Self::Jules => "🚀",
            Self::OpenCode => "🔓",
            Self::Pi => "🥧",
        }
    }

    pub fn config_flag_name(&self) -> &'static str {
        match self {
            Self::Claude => "claude-dir",
            Self::ClaudeCodeRouter => "claude-code-router-dir",
            Self::CodeRabbit => "coderabbit-dir",
            Self::Copilot => "copilot-dir",
            Self::Cursor => "cursor-dir",
            Self::Gemini => "gemini-dir",
            Self::Codex => "codex-dir",
            Self::Jules => "jules-dir",
            Self::OpenCode => "opencode-dir",
            Self::Pi => "pi-dir",
        }
    }

    pub fn requires_server_start(&self) -> bool {
        match self {
            Self::ClaudeCodeRouter => claude_code_router::REQUIRES_SERVER_START,
            _ => false,
        }
    }

    pub fn server_start_command(&self) -> Option<&'static str> {
        match self {
            Self::ClaudeCodeRouter => Some(claude_code_router::SERVER_START_COMMAND),
            _ => None,
        }
    }

    pub fn main_command(&self) -> Option<&'static str> {
        match self {
            Self::ClaudeCodeRouter => Some(claude_code_router::MAIN_COMMAND),
            _ => None,
        }
    }

    pub fn needs_auth(&self, home_dir: &std::path::Path) -> bool {
        let cred_path = home_dir.join(self.auth_credential_path());

        if !cred_path.exists() {
            return true;
        }

        if cred_path.is_file() {
            if let Ok(metadata) = std::fs::metadata(&cred_path) {
                if metadata.len() == 0 {
                    return true;
                }
            }
        }

        if cred_path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&cred_path) {
                if entries.count() == 0 {
                    return true;
                }
            }
        }

        false
    }

    pub fn is_config_flag_set(&self, flags: &AgentConfigFlags) -> bool {
        flags.agent_configs || flags.get(self)
    }

    pub fn validate_config_flags(&self, flags: &AgentConfigFlags) -> Result<(), String> {
        if flags.agent_configs {
            return Ok(());
        }

        let incompatible_flags: Vec<&str> = ALL_AGENTS
            .iter()
            .filter(|a| *a != self && flags.get(a))
            .map(|a| a.config_flag_name())
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

#[derive(Debug, Default)]
pub struct AgentConfigFlags {
    pub claude_dir: bool,
    pub claude_code_router_dir: bool,
    pub coderabbit_dir: bool,
    pub copilot_dir: bool,
    pub cursor_dir: bool,
    pub gemini_dir: bool,
    pub codex_dir: bool,
    pub jules_dir: bool,
    pub opencode_dir: bool,
    pub pi_dir: bool,
    pub agent_configs: bool,
}

impl AgentConfigFlags {
    pub fn get(&self, agent: &Agent) -> bool {
        match agent {
            Agent::Claude => self.claude_dir,
            Agent::ClaudeCodeRouter => self.claude_code_router_dir,
            Agent::CodeRabbit => self.coderabbit_dir,
            Agent::Copilot => self.copilot_dir,
            Agent::Cursor => self.cursor_dir,
            Agent::Gemini => self.gemini_dir,
            Agent::Codex => self.codex_dir,
            Agent::Jules => self.jules_dir,
            Agent::OpenCode => self.opencode_dir,
            Agent::Pi => self.pi_dir,
        }
    }
}

impl fmt::Display for Agent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

pub fn extract_agent_from_jail_name(jail_name: &str) -> Option<Agent> {
    if !jail_name.starts_with("jail__") {
        return None;
    }

    jail_name.rsplit("__").next().and_then(Agent::from_str)
}

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
        assert_eq!(Agent::from_str("opencode"), Some(Agent::OpenCode));
        assert_eq!(Agent::from_str("unknown"), None);
        assert_eq!(Agent::from_str("CLAUDE"), Some(Agent::Claude));
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
        assert_eq!(Agent::OpenCode.command_name(), "opencode");
    }

    #[test]
    fn test_agent_normalized_name() {
        assert_eq!(Agent::Claude.normalized_name(), "claude");
        assert_eq!(
            Agent::ClaudeCodeRouter.normalized_name(),
            "claude-code-router"
        );
        assert_eq!(Agent::Cursor.normalized_name(), "cursor");
        assert_eq!(Agent::OpenCode.normalized_name(), "opencode");
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
        assert_eq!(
            extract_agent_from_jail_name("jail__test__abc12345__opencode"),
            Some(Agent::OpenCode)
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
        assert!(!Agent::OpenCode.has_auto_credentials());
    }

    #[test]
    fn test_agent_layer_name() {
        assert_eq!(Agent::Claude.layer_name(), "agent-claude");
        assert_eq!(
            Agent::ClaudeCodeRouter.layer_name(),
            "agent-claude-code-router"
        );
        assert_eq!(Agent::Cursor.layer_name(), "agent-cursor");
        assert_eq!(Agent::OpenCode.layer_name(), "agent-opencode");
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
        assert_eq!(Agent::OpenCode.auth_credential_path(), ".config/opencode");
    }

    #[test]
    fn test_agent_needs_auth() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let home_path = temp_dir.path();

        assert!(Agent::Codex.needs_auth(home_path));

        let codex_dir = home_path.join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        assert!(Agent::Codex.needs_auth(home_path));

        let cred_file = codex_dir.join("credentials.json");
        fs::write(cred_file, "{}").unwrap();
        assert!(!Agent::Codex.needs_auth(home_path));

        let claude_creds = home_path.join(".claude/.credentials.json");
        fs::create_dir_all(claude_creds.parent().unwrap()).unwrap();
        fs::write(&claude_creds, "").unwrap();
        assert!(Agent::Claude.needs_auth(home_path));

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
        assert_eq!(Agent::OpenCode.config_flag_name(), "opencode-dir");
    }

    #[test]
    fn test_validate_config_flags_matching_agent() {
        let flags = AgentConfigFlags {
            claude_dir: true,
            ..Default::default()
        };
        assert!(Agent::Claude.validate_config_flags(&flags).is_ok());

        let flags = AgentConfigFlags {
            copilot_dir: true,
            ..Default::default()
        };
        assert!(Agent::Copilot.validate_config_flags(&flags).is_ok());
    }

    #[test]
    fn test_validate_config_flags_mismatched_agent() {
        let flags = AgentConfigFlags {
            gemini_dir: true,
            ..Default::default()
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
        let flags = AgentConfigFlags {
            claude_dir: true,
            copilot_dir: true,
            gemini_dir: true,
            ..Default::default()
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
        let flags = AgentConfigFlags {
            claude_dir: true,
            claude_code_router_dir: true,
            copilot_dir: true,
            cursor_dir: true,
            gemini_dir: true,
            coderabbit_dir: true,
            codex_dir: true,
            jules_dir: true,
            opencode_dir: true,
            pi_dir: true,
            agent_configs: true,
        };
        assert!(Agent::Claude.validate_config_flags(&flags).is_ok());
        assert!(Agent::Copilot.validate_config_flags(&flags).is_ok());
        assert!(Agent::Cursor.validate_config_flags(&flags).is_ok());
        assert!(Agent::Gemini.validate_config_flags(&flags).is_ok());
        assert!(Agent::CodeRabbit.validate_config_flags(&flags).is_ok());
        assert!(Agent::Codex.validate_config_flags(&flags).is_ok());
        assert!(Agent::Jules.validate_config_flags(&flags).is_ok());
    }

    #[test]
    fn test_validate_config_flags_no_flags() {
        let flags = AgentConfigFlags::default();
        assert!(Agent::Claude.validate_config_flags(&flags).is_ok());
        assert!(Agent::Copilot.validate_config_flags(&flags).is_ok());
        assert!(Agent::Cursor.validate_config_flags(&flags).is_ok());
    }

    #[test]
    fn test_all_agents_have_from_str_roundtrip() {
        for agent in ALL_AGENTS {
            let name = agent.normalized_name();
            let parsed = Agent::from_str(name);
            assert_eq!(
                parsed,
                Some(*agent),
                "Agent::from_str({:?}) should return {:?}",
                name,
                agent
            );
        }
    }

    #[test]
    fn test_all_agents_have_emoji() {
        for agent in ALL_AGENTS {
            let emoji = agent.emoji();
            assert!(
                !emoji.is_empty(),
                "Agent {:?} should have a non-empty emoji",
                agent
            );
        }
    }

    #[test]
    fn test_all_agents_have_config_flag() {
        for agent in ALL_AGENTS {
            let flag = agent.config_flag_name();
            assert!(
                flag.ends_with("-dir"),
                "Agent {:?} config flag {:?} should end with -dir",
                agent,
                flag
            );
        }
    }

    #[test]
    fn test_agent_config_flags_get() {
        let flags = AgentConfigFlags {
            claude_dir: true,
            opencode_dir: true,
            ..Default::default()
        };
        assert!(flags.get(&Agent::Claude));
        assert!(flags.get(&Agent::OpenCode));
        assert!(!flags.get(&Agent::Copilot));
    }

    #[test]
    fn test_is_config_flag_set_with_agent_configs() {
        let flags = AgentConfigFlags {
            agent_configs: true,
            ..Default::default()
        };
        for agent in ALL_AGENTS {
            assert!(
                agent.is_config_flag_set(&flags),
                "Agent {:?} should report config flag set when agent_configs=true",
                agent
            );
        }
    }
}
