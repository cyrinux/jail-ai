pub const COMMAND_NAME: &str = "ccr";
pub const NORMALIZED_NAME: &str = "claude-code-router";
pub const DISPLAY_NAME: &str = "Claude Code Router";
pub const EMOJI: &str = "🔀";
pub const HAS_AUTO_CREDENTIALS: bool = false;
pub const CONFIG_DIR_PATHS: &[(&str, &str)] = &[
    (".claude", "/home/agent/.claude"),
    (".claude-code-router", "/home/agent/.claude-code-router"),
];
pub const SUPPORTS_AUTH_WORKFLOW: bool = false;
pub const AUTH_CREDENTIAL_PATH: &str = ".claude-code-router";
pub const CLI_ALIASES: &[&str] = &["ccr"];
pub const CONTAINERFILE: &str =
    include_str!("../../containerfiles/agent-claude-code-router.Containerfile");

pub const REQUIRES_SERVER_START: bool = true;
pub const SERVER_START_COMMAND: &str = "start";
pub const MAIN_COMMAND: &str = "code";
