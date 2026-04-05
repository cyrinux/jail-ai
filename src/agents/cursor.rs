pub const COMMAND_NAME: &str = "cursor-agent";
pub const NORMALIZED_NAME: &str = "cursor";
pub const DISPLAY_NAME: &str = "Cursor";
pub const EMOJI: &str = "➡️";
pub const HAS_AUTO_CREDENTIALS: bool = false;
pub const CONFIG_DIR_PATHS: &[(&str, &str)] = &[
    (".cursor", "/home/agent/.cursor"),
    (".config/cursor", "/home/agent/.config/cursor"),
];
pub const SUPPORTS_AUTH_WORKFLOW: bool = false;
pub const AUTH_CREDENTIAL_PATH: &str = ".cursor";
pub const CLI_ALIASES: &[&str] = &["cursor-agent"];
pub const CONTAINERFILE: &str = include_str!("../../containerfiles/agent-cursor.Containerfile");
