pub const COMMAND_NAME: &str = "opencode";
pub const NORMALIZED_NAME: &str = "opencode";
pub const DISPLAY_NAME: &str = "OpenCode";
pub const EMOJI: &str = "🔓";
pub const HAS_AUTO_CREDENTIALS: bool = false;
pub const CONFIG_DIR_PATHS: &[(&str, &str)] =
    &[(".config/opencode", "/home/agent/.config/opencode")];
pub const SUPPORTS_AUTH_WORKFLOW: bool = false;
pub const AUTH_CREDENTIAL_PATH: &str = ".config/opencode";
pub const CLI_ALIASES: &[&str] = &[];
pub const CONTAINERFILE: &str = include_str!("../../containerfiles/agent-opencode.Containerfile");
