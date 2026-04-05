use jail_ai::image_layers::{
    generate_project_hash, get_agent_project_image_name, get_containerfile_content,
    get_language_image_name, get_project_image_name, AWS_IMAGE_NAME, BASE_IMAGE_NAME,
    CPP_IMAGE_NAME, CSHARP_IMAGE_NAME, GCP_IMAGE_NAME, GOLANG_IMAGE_NAME, JAVA_IMAGE_NAME,
    NIX_IMAGE_NAME, NODEJS_IMAGE_NAME, PHP_IMAGE_NAME, PYTHON_IMAGE_NAME, RUST_IMAGE_NAME,
};
use jail_ai::ProjectType;

#[test]
fn test_get_language_image_name() {
    assert_eq!(get_language_image_name(&ProjectType::Rust), RUST_IMAGE_NAME);
    assert_eq!(
        get_language_image_name(&ProjectType::Golang),
        GOLANG_IMAGE_NAME
    );
    assert_eq!(
        get_language_image_name(&ProjectType::Python),
        PYTHON_IMAGE_NAME
    );
    assert_eq!(
        get_language_image_name(&ProjectType::NodeJS),
        NODEJS_IMAGE_NAME
    );
    assert_eq!(get_language_image_name(&ProjectType::Java), JAVA_IMAGE_NAME);
    assert_eq!(get_language_image_name(&ProjectType::Nix), NIX_IMAGE_NAME);
    assert_eq!(get_language_image_name(&ProjectType::Php), PHP_IMAGE_NAME);
    assert_eq!(get_language_image_name(&ProjectType::Cpp), CPP_IMAGE_NAME);
    assert_eq!(
        get_language_image_name(&ProjectType::CSharp),
        CSHARP_IMAGE_NAME
    );
    assert_eq!(get_language_image_name(&ProjectType::Aws), AWS_IMAGE_NAME);
    assert_eq!(get_language_image_name(&ProjectType::Gcp), GCP_IMAGE_NAME);
    assert_eq!(
        get_language_image_name(&ProjectType::Generic),
        BASE_IMAGE_NAME
    );
}

#[test]
fn test_get_agent_project_image_name() {
    assert_eq!(
        get_agent_project_image_name("claude", "abc12345"),
        "localhost/jail-ai-agent-claude:abc12345"
    );
    assert_eq!(
        get_agent_project_image_name("copilot", "def67890"),
        "localhost/jail-ai-agent-copilot:def67890"
    );
}

#[test]
fn test_generate_project_hash() {
    use std::path::PathBuf;

    let path1 = PathBuf::from("/tmp/project-a");
    let hash1 = generate_project_hash(&path1);

    assert_eq!(hash1.len(), 8);

    let hash2 = generate_project_hash(&path1);
    assert_eq!(hash1, hash2);

    let path2 = PathBuf::from("/tmp/project-b");
    let hash3 = generate_project_hash(&path2);
    assert_ne!(hash1, hash3);
}

#[test]
fn test_get_project_image_name() {
    assert_eq!(
        get_project_image_name("rust", "abc12345"),
        "localhost/jail-ai-rust:abc12345"
    );
    assert_eq!(
        get_project_image_name("python", "def67890"),
        "localhost/jail-ai-python:def67890"
    );
}

#[test]
fn test_get_containerfile_content() {
    assert!(get_containerfile_content("base").is_some());
    assert!(get_containerfile_content("golang").is_some());
    assert!(get_containerfile_content("rust").is_some());
    assert!(get_containerfile_content("python").is_some());
    assert!(get_containerfile_content("nodejs").is_some());
    assert!(get_containerfile_content("java").is_some());
    assert!(get_containerfile_content("nix").is_some());
    assert!(get_containerfile_content("php").is_some());
    assert!(get_containerfile_content("cpp").is_some());
    assert!(get_containerfile_content("csharp").is_some());
    assert!(get_containerfile_content("aws").is_some());
    assert!(get_containerfile_content("gcp").is_some());
    assert!(get_containerfile_content("agent-claude").is_some());
    assert!(get_containerfile_content("agent-pi").is_some());
    assert!(get_containerfile_content("unknown").is_none());
}
