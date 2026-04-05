use jail_ai::{
    backend::podman::PodmanBackend,
    config::{BackendType, JailConfig, NetworkConfig, PortMapping, ResourceLimits},
};

#[test]
fn test_build_run_args() {
    let backend = PodmanBackend::new();

    let config = JailConfig {
        name: "test".to_string(),
        backend: BackendType::Podman,
        base_image: "alpine:latest".to_string(),
        bind_mounts: vec![],
        environment: vec![("TEST".to_string(), "value".to_string())],
        network: NetworkConfig {
            enabled: false,
            private: true,
            host: false,
        },
        port_mappings: vec![],
        limits: ResourceLimits {
            memory_mb: Some(512),
            cpu_quota: Some(50),
        },
        upgrade: false,
        force_layers: Vec::new(),
        use_layered_images: true,
        isolated: false,
        verbose: false,
        pre_create_dirs: Vec::new(),
        no_nix: false,
        block_host: false,
        podman_socket: false,
    };

    let args = backend.build_run_args(&config);

    assert!(args.contains(&"run".to_string()));
    assert!(args.contains(&"--name".to_string()));
    assert!(args.contains(&"test".to_string()));
    assert!(args.contains(&"-m".to_string()));
    assert!(args.contains(&"512m".to_string()));
    assert!(args.contains(&"-e".to_string()));
    assert!(args.contains(&"TEST=value".to_string()));
    assert!(args.contains(&"test__home:/home/agent".to_string()));

    let config_agent = JailConfig {
        name: "jail__project__abc12345__claude".to_string(),
        ..config.clone()
    };

    let args_agent = backend.build_run_args(&config_agent);

    assert!(args_agent.contains(&"jail__project__abc12345__claude__home:/home/agent".to_string()));

    let config_copilot = JailConfig {
        name: "jail__project__abc12345__copilot".to_string(),
        ..config.clone()
    };

    let args_copilot = backend.build_run_args(&config_copilot);

    assert!(
        args_copilot.contains(&"jail__project__abc12345__copilot__home:/home/agent".to_string())
    );
    assert!(
        !args_copilot.contains(&"jail__project__abc12345__claude__home:/home/agent".to_string())
    );
}

#[test]
fn test_build_run_args_with_port_mappings() {
    let backend = PodmanBackend::new();
    let config = JailConfig {
        name: "test-jail".to_string(),
        backend: BackendType::Podman,
        base_image: "alpine:latest".to_string(),
        bind_mounts: vec![],
        environment: vec![],
        network: NetworkConfig {
            enabled: true,
            private: true,
            host: false,
        },
        port_mappings: vec![
            PortMapping {
                host_port: 8080,
                container_port: 80,
                protocol: "tcp".to_string(),
            },
            PortMapping {
                host_port: 5432,
                container_port: 5432,
                protocol: "tcp".to_string(),
            },
        ],
        limits: ResourceLimits {
            memory_mb: None,
            cpu_quota: None,
        },
        upgrade: false,
        force_layers: Vec::new(),
        use_layered_images: true,
        isolated: false,
        verbose: false,
        no_nix: false,
        pre_create_dirs: Vec::new(),
        block_host: false,
        podman_socket: false,
    };

    let args = backend.build_run_args(&config);

    assert!(args.contains(&"-p".to_string()));
    assert!(args.contains(&"8080:80/tcp".to_string()));
    assert!(args.contains(&"5432:5432/tcp".to_string()));
}

#[test]
fn test_build_run_args_port_mappings_require_network() {
    let backend = PodmanBackend::new();
    let config = JailConfig {
        name: "test-jail".to_string(),
        backend: BackendType::Podman,
        base_image: "alpine:latest".to_string(),
        bind_mounts: vec![],
        environment: vec![],
        network: NetworkConfig {
            enabled: false,
            private: true,
            host: false,
        },
        port_mappings: vec![PortMapping {
            host_port: 8080,
            container_port: 80,
            protocol: "tcp".to_string(),
        }],
        limits: ResourceLimits {
            memory_mb: None,
            cpu_quota: None,
        },
        upgrade: false,
        force_layers: Vec::new(),
        use_layered_images: true,
        isolated: false,
        verbose: false,
        no_nix: false,
        pre_create_dirs: Vec::new(),
        block_host: false,
        podman_socket: false,
    };

    let args = backend.build_run_args(&config);

    let port_args_count = args.iter().filter(|&arg| arg == "-p").count();
    assert_eq!(port_args_count, 0);
}

#[test]
fn test_list_all_filters_jail_prefix() {
    let names = vec![
        "jail__project__def67890__claude",
        "other-container",
        "jail__another__xyz12345__copilot",
        "my-container",
    ];

    let filtered: Vec<String> = names
        .into_iter()
        .filter(|name| name.starts_with("jail__"))
        .map(|s| s.to_string())
        .collect();

    assert_eq!(filtered.len(), 2);
    assert!(filtered.contains(&"jail__project__def67890__claude".to_string()));
    assert!(filtered.contains(&"jail__another__xyz12345__copilot".to_string()));
}

#[test]
fn test_image_uses_nix() {
    assert!(PodmanBackend::image_uses_nix(
        "localhost/jail-ai-nix:latest"
    ));

    assert!(PodmanBackend::image_uses_nix(
        "localhost/jail-ai-agent-claude:base-nix"
    ));
    assert!(PodmanBackend::image_uses_nix(
        "localhost/jail-ai-agent-claude:base-nix-rust"
    ));
    assert!(PodmanBackend::image_uses_nix(
        "localhost/jail-ai-agent-claude:base-rust-nix"
    ));
    assert!(PodmanBackend::image_uses_nix(
        "localhost/jail-ai-agent-jules:base-rust-nix-nodejs"
    ));

    assert!(!PodmanBackend::image_uses_nix(
        "localhost/jail-ai-agent-claude:base"
    ));
    assert!(!PodmanBackend::image_uses_nix(
        "localhost/jail-ai-agent-claude:base-rust"
    ));
    assert!(!PodmanBackend::image_uses_nix(
        "localhost/jail-ai-agent-claude:base-rust-nodejs"
    ));
    assert!(!PodmanBackend::image_uses_nix("alpine:latest"));

    assert!(!PodmanBackend::image_uses_nix(
        "localhost/phoenix-app:latest"
    ));
    assert!(!PodmanBackend::image_uses_nix(
        "localhost/jail-ai-agent-claude:base-unix"
    ));
}

#[test]
fn test_extract_base_name() {
    assert_eq!(
        PodmanBackend::extract_base_name("jail__project__abc12345__claude"),
        "jail__project__abc12345"
    );
    assert_eq!(
        PodmanBackend::extract_base_name("jail__project__abc12345__copilot"),
        "jail__project__abc12345"
    );
    assert_eq!(
        PodmanBackend::extract_base_name("jail__project__def67890__cursor"),
        "jail__project__def67890"
    );
    assert_eq!(
        PodmanBackend::extract_base_name("jail__project__12345678__gemini"),
        "jail__project__12345678"
    );
    assert_eq!(
        PodmanBackend::extract_base_name("jail__myproject__abcdef12__jules"),
        "jail__myproject__abcdef12"
    );
    assert_eq!(
        PodmanBackend::extract_base_name("jail__test__fedcba98__codex"),
        "jail__test__fedcba98"
    );

    assert_eq!(
        PodmanBackend::extract_base_name("jail__project__abc12345__newagent"),
        "jail__project__abc12345"
    );

    assert_eq!(PodmanBackend::extract_base_name("test"), "test");
}

#[test]
fn test_build_run_args_with_nix_volume() {
    let backend = PodmanBackend::new();

    let config_with_nix = JailConfig {
        name: "jail__project__abc12345__claude".to_string(),
        backend: BackendType::Podman,
        base_image: "localhost/jail-ai-agent-claude:base-nix-rust".to_string(),
        bind_mounts: vec![],
        environment: vec![],
        network: NetworkConfig {
            enabled: true,
            private: true,
            host: false,
        },
        port_mappings: vec![],
        limits: ResourceLimits {
            memory_mb: None,
            cpu_quota: None,
        },
        upgrade: false,
        force_layers: Vec::new(),
        use_layered_images: true,
        isolated: false,
        verbose: false,
        no_nix: false,
        pre_create_dirs: Vec::new(),
        block_host: false,
        podman_socket: false,
    };

    let args = backend.build_run_args(&config_with_nix);

    assert!(args.contains(&"jail__project__abc12345__claude__home:/home/agent".to_string()));
    assert!(args.contains(&"jail__project__abc12345__nix:/nix".to_string()));

    let config_with_copilot = JailConfig {
        name: "jail__project__abc12345__copilot".to_string(),
        ..config_with_nix.clone()
    };

    let args_copilot = backend.build_run_args(&config_with_copilot);

    assert!(
        args_copilot.contains(&"jail__project__abc12345__copilot__home:/home/agent".to_string())
    );
    assert!(args_copilot.contains(&"jail__project__abc12345__nix:/nix".to_string()));

    assert!(
        !args_copilot.contains(&"jail__project__abc12345__claude__home:/home/agent".to_string())
    );

    let config_without_nix = JailConfig {
        base_image: "alpine:latest".to_string(),
        ..config_with_nix
    };

    let args = backend.build_run_args(&config_without_nix);

    assert!(args.contains(&"jail__project__abc12345__claude__home:/home/agent".to_string()));
    assert!(!args.contains(&"jail__project__abc12345__nix:/nix".to_string()));

    assert!(!args.iter().any(|arg| arg.contains("jail-ai-nix-store")));
}
