use jail_ai::{cli::Commands, config, Cli, Parser};

#[test]
fn test_parse_backend() {
    assert!(matches!(
        Commands::parse_backend("podman"),
        Ok(config::BackendType::Podman)
    ));
    assert!(matches!(
        Commands::parse_backend("pod"),
        Ok(config::BackendType::Podman)
    ));
    assert!(matches!(
        Commands::parse_backend("container-app"),
        Ok(config::BackendType::ContainerApp)
    ));
    assert!(matches!(
        Commands::parse_backend("container"),
        Ok(config::BackendType::ContainerApp)
    ));
    assert!(matches!(
        Commands::parse_backend("apple"),
        Ok(config::BackendType::ContainerApp)
    ));
    assert!(Commands::parse_backend("invalid").is_err());
}

#[test]
fn test_parse_mount() {
    let mount = Commands::parse_mount("/tmp:/dst:ro").unwrap();
    assert_eq!(mount.source, std::path::PathBuf::from("/tmp"));
    assert_eq!(mount.target, std::path::PathBuf::from("/dst"));
    assert!(mount.readonly);

    let mount = Commands::parse_mount("/tmp:/dst").unwrap();
    assert!(!mount.readonly);

    assert!(Commands::parse_mount("invalid").is_err());

    assert!(Commands::parse_mount("/:/dst").is_err());
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/agent".to_string());
    assert!(Commands::parse_mount(&format!("{}:/dst", home)).is_err());
    let home_config = format!("{}/.config:/dst", home);
    if std::path::Path::new(&home_config.split(':').next().unwrap()).exists() {
        assert!(Commands::parse_mount(&home_config).is_ok());
    }
}

#[test]
fn test_parse_env() {
    let (key, value) = Commands::parse_env("KEY=VALUE").unwrap();
    assert_eq!(key, "KEY");
    assert_eq!(value, "VALUE");

    assert!(Commands::parse_env("INVALID").is_err());
}

#[test]
fn test_sanitize_jail_name() {
    assert_eq!(Commands::sanitize_jail_name(".dotfiles"), "dotfiles");
    assert_eq!(Commands::sanitize_jail_name("...dotfiles"), "dotfiles");

    assert_eq!(Commands::sanitize_jail_name("my@project"), "my-project");
    assert_eq!(Commands::sanitize_jail_name("test project"), "test-project");

    assert_eq!(
        Commands::sanitize_jail_name("my_project.v2"),
        "my_project.v2"
    );
    assert_eq!(
        Commands::sanitize_jail_name("my-project-v2"),
        "my-project-v2"
    );

    assert_eq!(Commands::sanitize_jail_name("-myproject"), "myproject");
    assert_eq!(Commands::sanitize_jail_name("_myproject"), "myproject");

    assert_eq!(Commands::sanitize_jail_name("..."), "default");
    assert_eq!(Commands::sanitize_jail_name(""), "default");
    assert_eq!(Commands::sanitize_jail_name("---"), "default");

    assert_eq!(Commands::sanitize_jail_name(".project"), "project");

    assert_eq!(Commands::sanitize_jail_name("myproject"), "myproject");
    assert_eq!(Commands::sanitize_jail_name("MyProject123"), "MyProject123");
}

#[test]
fn test_generate_jail_name() {
    use std::path::PathBuf;

    let path = PathBuf::from("/tmp/test-project");
    let name = Commands::generate_jail_name(&path);

    assert!(name.starts_with("jail__"));

    assert!(name.contains("test-project__"));

    let name2 = Commands::generate_jail_name(&path);
    assert_eq!(name, name2);

    let path2 = PathBuf::from("/tmp/another-project");
    let name3 = Commands::generate_jail_name(&path2);
    assert_ne!(name, name3);
}

#[test]
fn test_generate_jail_name_sanitization() {
    use std::path::PathBuf;

    let path = PathBuf::from("/tmp/my-project@2024");
    let name = Commands::generate_jail_name(&path);

    assert!(name.contains("my-project-2024__"));
}

#[test]
fn test_host_network_flag_parsing() {
    let args = vec!["jail-ai", "claude", "--host-network"];
    let cli = Cli::try_parse_from(args).unwrap();

    match cli.command {
        Some(Commands::Claude { common, .. }) => {
            assert!(common.host_network);
            assert!(!common.no_network);
        }
        _ => panic!("Expected Claude command"),
    }
}

#[test]
fn test_host_network_conflicts_with_no_network() {
    let args = vec!["jail-ai", "claude", "--host-network", "--no-network"];
    assert!(Cli::try_parse_from(args).is_err());
}

#[test]
fn test_host_network_flag_create_command() {
    let args = vec!["jail-ai", "create", "test-jail", "--host-network"];
    let cli = Cli::try_parse_from(args).unwrap();

    match cli.command {
        Some(Commands::Create {
            host_network,
            no_network,
            ..
        }) => {
            assert!(host_network);
            assert!(!no_network);
        }
        _ => panic!("Expected Create command"),
    }
}

#[test]
fn test_auth_flag_parsing() {
    let args = vec!["jail-ai", "codex", "--auth", "--config-dir"];
    let cli = Cli::try_parse_from(args).unwrap();

    match cli.command {
        Some(Commands::Codex { common, .. }) => {
            assert!(common.auth);
        }
        _ => panic!("Expected Codex command"),
    }
}

#[test]
fn test_auth_flag_optional() {
    let args = vec!["jail-ai", "codex", "--config-dir"];
    let cli = Cli::try_parse_from(args).unwrap();

    match cli.command {
        Some(Commands::Codex { common, .. }) => {
            assert!(!common.auth);
        }
        _ => panic!("Expected Codex command"),
    }
}

#[test]
fn test_parse_port() {
    let port = Commands::parse_port("8080:80").unwrap();
    assert_eq!(port.host_port, 8080);
    assert_eq!(port.container_port, 80);
    assert_eq!(port.protocol, "tcp");

    let port = Commands::parse_port("8080:80/tcp").unwrap();
    assert_eq!(port.host_port, 8080);
    assert_eq!(port.container_port, 80);
    assert_eq!(port.protocol, "tcp");

    let port = Commands::parse_port("5432:5432/udp").unwrap();
    assert_eq!(port.host_port, 5432);
    assert_eq!(port.container_port, 5432);
    assert_eq!(port.protocol, "udp");

    let port = Commands::parse_port("5432:5432").unwrap();
    assert_eq!(port.host_port, 5432);
    assert_eq!(port.container_port, 5432);
    assert_eq!(port.protocol, "tcp");

    assert!(Commands::parse_port("invalid").is_err());
    assert!(Commands::parse_port("8080").is_err());
    assert!(Commands::parse_port("8080:80:90").is_err());

    assert!(Commands::parse_port("8080:80/http").is_err());

    assert!(Commands::parse_port("invalid:80").is_err());
    assert!(Commands::parse_port("8080:invalid").is_err());
    assert!(Commands::parse_port("70000:80").is_err());
}
