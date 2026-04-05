pub const COMMAND_NAME: &str = "coderabbit";
pub const NORMALIZED_NAME: &str = "coderabbit";
pub const DISPLAY_NAME: &str = "CodeRabbit";
pub const EMOJI: &str = "🐰";
pub const HAS_AUTO_CREDENTIALS: bool = false;
pub const CONFIG_DIR_PATHS: &[(&str, &str)] = &[(".coderabbit", "/home/agent/.coderabbit")];
pub const SUPPORTS_AUTH_WORKFLOW: bool = true;
pub const AUTH_CREDENTIAL_PATH: &str = ".coderabbit";
pub const CLI_ALIASES: &[&str] = &["code-rabbit"];
pub const CONTAINERFILE: &str =
    include_str!("../../containerfiles/agent-coderabbit.Containerfile");
