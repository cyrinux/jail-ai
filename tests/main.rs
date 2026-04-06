use jail_ai::{cli, config::JailConfig, error, upgrade_single_jail};

#[tokio::test]
async fn test_jail_config_serialization() {
    let config = JailConfig {
        name: "test".to_string(),
        backend: jail_ai::config::BackendType::Podman,
        ..Default::default()
    };

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: JailConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(config.name, deserialized.name);
    assert_eq!(config.backend, deserialized.backend);
}

#[test]
fn test_find_jails_for_directory_filters_correctly() {
    use std::path::PathBuf;

    let path = PathBuf::from("/tmp/test-project");
    let base_name = cli::Commands::generate_jail_name(&path);

    let all_jails = vec![
        format!("{base_name}__claude"),
        format!("{base_name}__copilot"),
        format!("{base_name}__cursor"),
        "jail__other__12345678__claude".to_string(),
    ];

    let matching_jails: Vec<String> = all_jails
        .into_iter()
        .filter(|name| name.starts_with(&base_name) && name.len() > base_name.len())
        .collect();

    assert_eq!(matching_jails.len(), 3);
    assert!(matching_jails.contains(&format!("{base_name}__claude")));
    assert!(matching_jails.contains(&format!("{base_name}__copilot")));
    assert!(matching_jails.contains(&format!("{base_name}__cursor")));
}

#[test]
fn test_resolve_jail_name_logic() {
    use std::path::PathBuf;

    let path = PathBuf::from("/tmp/test-project");
    let base_name = cli::Commands::generate_jail_name(&path);

    assert!(!base_name.ends_with("__claude"));
    assert!(!base_name.ends_with("__copilot"));
    assert!(!base_name.ends_with("__cursor"));

    assert!(base_name.starts_with("jail__"));
    assert!(base_name.contains("test-project__"));
}

#[tokio::test]
async fn test_upgrade_single_jail_nonexistent() {
    let result: Result<(), error::JailError> =
        upgrade_single_jail("nonexistent-jail", None, true, false).await;
    assert!(result.is_err());
    if let Err(e) = result {
        match e {
            error::JailError::NotFound(_) => {}
            _ => panic!("Expected NotFound error, got {:?}", e),
        }
    }
}
