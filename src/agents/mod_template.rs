macro_rules! define_agent_enum {
    ($(($Variant:ident, $mod:ident, $cli_name:literal)),* $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Agent {
            $($Variant,)*
        }

        pub const ALL_AGENTS: &[Agent] = &[$(Agent::$Variant,)*];

        impl Agent {
            #[allow(clippy::should_implement_trait)]
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

macro_rules! define_cli_commands {
    ($(($Variant:ident, $mod:ident, $cli_name:literal)),* $(,)?) => {
        #[derive(clap::Subcommand, Debug, Clone)]
        #[command(subcommand_help_heading = "Agent Commands")]
        pub enum AgentCommands {
            $(
                #[command(name = $cli_name)]
                $Variant {
                    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
                    args: Vec<String>,
                },
            )*
        }

        impl AgentCommands {
            pub fn to_agent(&self) -> Agent {
                match self {
                    $(Self::$Variant { .. } => Agent::$Variant,)*
                }
            }

            pub fn args(&self) -> Vec<String> {
                match self {
                    $(Self::$Variant { args } => args.clone(),)*
                }
            }
        }
    };
}

for_each_agent!(define_cli_commands);

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
