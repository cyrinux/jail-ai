use jail_ai::agents::{extract_agent_from_jail_name, get_agent_containerfile, Agent, ALL_AGENTS};

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
    assert_eq!(Agent::from_str("opencode"), Some(Agent::Opencode));
    assert_eq!(Agent::from_str("coderabbit"), Some(Agent::Coderabbit));
    assert_eq!(Agent::from_str("code-rabbit"), Some(Agent::Coderabbit));
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
    assert_eq!(Agent::Opencode.command_name(), "opencode");
}

#[test]
fn test_agent_normalized_name() {
    assert_eq!(Agent::Claude.normalized_name(), "claude");
    assert_eq!(
        Agent::ClaudeCodeRouter.normalized_name(),
        "claude-code-router"
    );
    assert_eq!(Agent::Cursor.normalized_name(), "cursor");
    assert_eq!(Agent::Opencode.normalized_name(), "opencode");
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
        Some(Agent::Opencode)
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
    assert!(!Agent::Opencode.has_auto_credentials());
}

#[test]
fn test_agent_layer_name() {
    assert_eq!(Agent::Claude.layer_name(), "agent-claude");
    assert_eq!(
        Agent::ClaudeCodeRouter.layer_name(),
        "agent-claude-code-router"
    );
    assert_eq!(Agent::Cursor.layer_name(), "agent-cursor");
    assert_eq!(Agent::Opencode.layer_name(), "agent-opencode");
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
    assert_eq!(Agent::Opencode.auth_credential_path(), ".config/opencode");
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
