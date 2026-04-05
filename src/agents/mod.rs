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

/// ─── AGENT REGISTRY ─────────────────────────────────────────────────
/// To add a new agent, add ONE line here. All enum variants, match arms,
/// CLI subcommands, image layer lookups, and config flag wiring are
/// generated automatically from this list.
///
/// Format: (VariantName, module_name, "cli-name")
///   - VariantName: PascalCase enum variant
///   - module_name: snake_case module in src/agents/
///   - "cli-name":  subcommand name on the CLI (e.g. `jail-ai claude`)
///
/// Everything else (command binary, display name, emoji, config paths,
/// aliases, containerfile) lives in the agent's own module file.
#[macro_export]
macro_rules! for_each_agent {
    ($callback:ident) => {
        $callback! {
            (Claude, claude, "claude"),
            (ClaudeCodeRouter, claude_code_router, "claude-code-router"),
            (CodeRabbit, coderabbit, "coderabbit"),
            (Copilot, copilot, "copilot"),
            (Cursor, cursor, "cursor"),
            (Gemini, gemini, "gemini"),
            (Codex, codex, "codex"),
            (Jules, jules, "jules"),
            (OpenCode, opencode, "opencode"),
            (Pi, pi, "pi"),
        }
    };
}

macro_rules! define_agent_enum {
    ($(($Variant:ident, $mod:ident, $cli_name:literal)),* $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Agent {
            $($Variant,)*
        }

        pub const ALL_AGENTS: &[Agent] = &[$(Agent::$Variant,)*];

        impl Agent {
            pub fn from_str(s: &str) -> Option<Self> {
                let s_lower = s.to_lowercase();
                $(
                    if s_lower == $mod::NORMALIZED_NAME
                        || $mod::CLI_ALIASES.contains(&s_lower.as_str())
                    {
                        return Some(Self::$Variant);
                    }
                )*
                None
            }

            pub fn command_name(&self) -> &'static str {
                match self { $(Self::$Variant => $mod::COMMAND_NAME,)* }
            }

            pub fn normalized_name(&self) -> &'static str {
                match self { $(Self::$Variant => $mod::NORMALIZED_NAME,)* }
            }

            pub fn display_name(&self) -> &'static str {
                match self { $(Self::$Variant => $mod::DISPLAY_NAME,)* }
            }

            pub fn emoji(&self) -> &'static str {
                match self { $(Self::$Variant => $mod::EMOJI,)* }
            }

            pub fn has_auto_credentials(&self) -> bool {
                match self { $(Self::$Variant => $mod::HAS_AUTO_CREDENTIALS,)* }
            }

            pub fn config_dir_paths(&self) -> Vec<(&'static str, &'static str)> {
                match self { $(Self::$Variant => $mod::CONFIG_DIR_PATHS.to_vec(),)* }
            }

            pub fn supports_auth_workflow(&self) -> bool {
                match self { $(Self::$Variant => $mod::SUPPORTS_AUTH_WORKFLOW,)* }
            }

            pub fn auth_credential_path(&self) -> &'static str {
                match self { $(Self::$Variant => $mod::AUTH_CREDENTIAL_PATH,)* }
            }

            pub fn containerfile(&self) -> &'static str {
                match self { $(Self::$Variant => $mod::CONTAINERFILE,)* }
            }
        }
    };
}

for_each_agent!(define_agent_enum);

impl Agent {
    pub fn layer_name(&self) -> String {
        format!("agent-{}", self.normalized_name())
    }

    #[allow(dead_code)]
    pub fn config_flag_name(&self) -> String {
        format!("{}-dir", self.normalized_name())
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

pub fn get_agent_containerfile(layer: &str) -> Option<&'static str> {
    let name = layer.strip_prefix("agent-")?;
    let agent = Agent::from_str(name)?;
    Some(agent.containerfile())
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
        assert_eq!(Agent::from_str("coderabbit"), Some(Agent::CodeRabbit));
        assert_eq!(Agent::from_str("code-rabbit"), Some(Agent::CodeRabbit));
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
            assert!(
                !agent.emoji().is_empty(),
                "Agent {:?} should have a non-empty emoji",
                agent
            );
        }
    }

    #[test]
    fn test_all_agents_have_containerfile() {
        for agent in ALL_AGENTS {
            assert!(
                !agent.containerfile().is_empty(),
                "Agent {:?} should have a non-empty containerfile",
                agent
            );
        }
    }

    #[test]
    fn test_get_agent_containerfile() {
        assert!(get_agent_containerfile("agent-claude").is_some());
        assert!(get_agent_containerfile("agent-opencode").is_some());
        assert!(get_agent_containerfile("agent-pi").is_some());
        assert!(get_agent_containerfile("agent-unknown").is_none());
        assert!(get_agent_containerfile("base").is_none());
    }
}
