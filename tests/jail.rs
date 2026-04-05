use jail_ai::{config::BackendType, jail::JailBuilder};

#[test]
fn test_jail_builder() {
    let manager = JailBuilder::new("test-jail")
        .backend(BackendType::Podman)
        .base_image("alpine:latest")
        .env("TEST", "value")
        .memory_limit(1024)
        .cpu_quota(75)
        .build();

    let config = manager.config();
    assert_eq!(config.name, "test-jail");
    assert_eq!(config.backend, BackendType::Podman);
    assert_eq!(config.base_image, "alpine:latest");
    assert_eq!(config.limits.memory_mb, Some(1024));
    assert_eq!(config.limits.cpu_quota, Some(75));
    assert_eq!(config.environment.len(), 1);
}

#[test]
fn test_jail_builder_no_nix() {
    let manager = JailBuilder::new("test-jail").no_nix(true).build();

    let config = manager.config();
    assert!(config.no_nix);

    let manager = JailBuilder::new("test-jail").no_nix(false).build();

    let config = manager.config();
    assert!(!config.no_nix);
}
