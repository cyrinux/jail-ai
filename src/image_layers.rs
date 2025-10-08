use crate::error::{JailError, Result};
use crate::project_detection::{detect_project_type, ProjectType};
use sha2::{Digest, Sha256};
use std::path::Path;
use tokio::process::Command;
use tracing::{debug, info};

/// Base image layers (shared across all projects with :latest tag)
const BASE_IMAGE_NAME: &str = "localhost/jail-ai-base:latest";
const GOLANG_IMAGE_NAME: &str = "localhost/jail-ai-golang:latest";
const RUST_IMAGE_NAME: &str = "localhost/jail-ai-rust:latest";
const PYTHON_IMAGE_NAME: &str = "localhost/jail-ai-python:latest";
const NODEJS_IMAGE_NAME: &str = "localhost/jail-ai-nodejs:latest";
const JAVA_IMAGE_NAME: &str = "localhost/jail-ai-java:latest";

/// Containerfiles embedded from the repository
const BASE_CONTAINERFILE: &str = include_str!("../containerfiles/base.Containerfile");
const GOLANG_CONTAINERFILE: &str = include_str!("../containerfiles/golang.Containerfile");
const RUST_CONTAINERFILE: &str = include_str!("../containerfiles/rust.Containerfile");
const PYTHON_CONTAINERFILE: &str = include_str!("../containerfiles/python.Containerfile");
const NODEJS_CONTAINERFILE: &str = include_str!("../containerfiles/nodejs.Containerfile");
const JAVA_CONTAINERFILE: &str = include_str!("../containerfiles/java.Containerfile");
const AGENT_CLAUDE_CONTAINERFILE: &str =
    include_str!("../containerfiles/agent-claude.Containerfile");
const AGENT_COPILOT_CONTAINERFILE: &str =
    include_str!("../containerfiles/agent-copilot.Containerfile");
const AGENT_CURSOR_CONTAINERFILE: &str =
    include_str!("../containerfiles/agent-cursor.Containerfile");
const AGENT_GEMINI_CONTAINERFILE: &str =
    include_str!("../containerfiles/agent-gemini.Containerfile");
const AGENT_CODEX_CONTAINERFILE: &str = include_str!("../containerfiles/agent-codex.Containerfile");

/// Generate a project identifier hash from workspace path
fn generate_project_hash(workspace_path: &Path) -> String {
    let abs_path = workspace_path
        .canonicalize()
        .unwrap_or_else(|_| workspace_path.to_path_buf());

    let mut hasher = Sha256::new();
    hasher.update(abs_path.to_string_lossy().as_bytes());
    let hash = hasher.finalize();
    let hash_hex = hex::encode(hash);

    // Use first 8 characters for readability
    hash_hex[..8].to_string()
}

/// Get the shared language image name (with :latest tag)
fn get_language_image_name(project_type: &ProjectType) -> &'static str {
    match project_type {
        ProjectType::Rust => RUST_IMAGE_NAME,
        ProjectType::Golang => GOLANG_IMAGE_NAME,
        ProjectType::Python => PYTHON_IMAGE_NAME,
        ProjectType::NodeJS => NODEJS_IMAGE_NAME,
        ProjectType::Java => JAVA_IMAGE_NAME,
        ProjectType::Multi(_) | ProjectType::Generic => BASE_IMAGE_NAME,
    }
}

/// Get the project-specific final image name
fn get_project_image_name(layer_type: &str, project_hash: &str) -> String {
    format!("localhost/jail-ai-{layer_type}:{project_hash}")
}

/// Get the project-specific agent image name
fn get_agent_project_image_name(agent_name: &str, project_hash: &str) -> String {
    format!("localhost/jail-ai-agent-{agent_name}:{project_hash}")
}

/// Get the Containerfile content for a layer
fn get_containerfile_content(layer: &str) -> Option<&'static str> {
    match layer {
        "base" => Some(BASE_CONTAINERFILE),
        "golang" => Some(GOLANG_CONTAINERFILE),
        "rust" => Some(RUST_CONTAINERFILE),
        "python" => Some(PYTHON_CONTAINERFILE),
        "nodejs" => Some(NODEJS_CONTAINERFILE),
        "java" => Some(JAVA_CONTAINERFILE),
        "agent-claude" => Some(AGENT_CLAUDE_CONTAINERFILE),
        "agent-copilot" => Some(AGENT_COPILOT_CONTAINERFILE),
        "agent-cursor" => Some(AGENT_CURSOR_CONTAINERFILE),
        "agent-gemini" => Some(AGENT_GEMINI_CONTAINERFILE),
        "agent-codex" => Some(AGENT_CODEX_CONTAINERFILE),
        _ => None,
    }
}

/// Check if an image exists locally
pub async fn image_exists(image_name: &str) -> Result<bool> {
    let mut cmd = Command::new("podman");
    cmd.arg("image").arg("exists").arg(image_name);

    match cmd.output().await {
        Ok(output) => Ok(output.status.success()),
        Err(_) => Ok(false),
    }
}

/// Build a shared layer image (with :latest tag)
async fn build_shared_layer(layer_name: &str, base_image: Option<&str>) -> Result<String> {
    let image_name = match layer_name {
        "base" => BASE_IMAGE_NAME.to_string(),
        "golang" => GOLANG_IMAGE_NAME.to_string(),
        "rust" => RUST_IMAGE_NAME.to_string(),
        "python" => PYTHON_IMAGE_NAME.to_string(),
        "nodejs" => NODEJS_IMAGE_NAME.to_string(),
        "java" => JAVA_IMAGE_NAME.to_string(),
        _ => {
            return Err(JailError::Backend(format!(
                "Unknown shared layer: {}",
                layer_name
            )))
        }
    };

    // Check if image already exists
    if image_exists(&image_name).await? {
        debug!("Shared layer {} already exists", image_name);
        return Ok(image_name);
    }

    build_image_from_containerfile(layer_name, base_image, &image_name).await
}

/// Internal function to build an image from a Containerfile
async fn build_image_from_containerfile(
    layer_name: &str,
    base_image: Option<&str>,
    image_tag: &str,
) -> Result<String> {
    info!("Building image: {} -> {}", layer_name, image_tag);

    let containerfile_content = get_containerfile_content(layer_name).ok_or_else(|| {
        JailError::Backend(format!(
            "No Containerfile found for layer: {}",
            layer_name
        ))
    })?;

    // Create a temporary file for the Containerfile
    let temp_dir = tempfile::tempdir()
        .map_err(|e| JailError::Backend(format!("Failed to create temp dir: {}", e)))?;
    let containerfile_path = temp_dir.path().join("Containerfile");
    tokio::fs::write(&containerfile_path, containerfile_content)
        .await
        .map_err(|e| JailError::Backend(format!("Failed to write Containerfile: {}", e)))?;

    // Build command
    let mut cmd = Command::new("podman");
    cmd.arg("build").arg("-t").arg(image_tag);

    // Add base image build arg if provided
    if let Some(base) = base_image {
        cmd.arg("--build-arg").arg(format!("BASE_IMAGE={}", base));
    }

    cmd.arg("-f")
        .arg(&containerfile_path)
        .arg(temp_dir.path());

    debug!("Running build command: {:?}", cmd);

    use std::process::Stdio;
    let status = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .map_err(|e| JailError::Backend(format!("Failed to execute build command: {}", e)))?;

    if !status.success() {
        return Err(JailError::Backend(format!(
            "Failed to build layer {}, build command exited with status: {}",
            layer_name, status
        )));
    }

    info!("Successfully built: {}", image_tag);
    Ok(image_tag.to_string())
}

/// Build the complete image stack for a project
pub async fn build_project_image(
    workspace_path: &Path,
    agent_name: Option<&str>,
    force_rebuild: bool,
) -> Result<String> {
    // Generate project-specific identifier
    let project_hash = generate_project_hash(workspace_path);
    info!("Project hash: {}", project_hash);

    // Detect project type
    let project_type = detect_project_type(workspace_path);
    info!("Detected project type: {:?}", project_type);

    // Step 1: Build base layer (shared :latest)
    let base_image = if force_rebuild || !image_exists(BASE_IMAGE_NAME).await? {
        info!("Building base layer...");
        build_shared_layer("base", None).await?
    } else {
        debug!("Base layer already exists");
        BASE_IMAGE_NAME.to_string()
    };

    // Step 2: Build language layer (shared :latest) if not generic
    let language_image = match project_type {
        ProjectType::Generic => base_image.clone(),
        ProjectType::Multi(ref types) => {
            let mut current_image = base_image.clone();
            for lang_type in types {
                let layer_name = lang_type.language_layer();
                let lang_image_name = get_language_image_name(lang_type);
                current_image = if force_rebuild || !image_exists(lang_image_name).await? {
                    build_shared_layer(layer_name, Some(&current_image)).await?
                } else {
                    lang_image_name.to_string()
                };
            }
            current_image
        }
        _ => {
            let layer_name = project_type.language_layer();
            let lang_image_name = get_language_image_name(&project_type);
            if force_rebuild || !image_exists(lang_image_name).await? {
                build_shared_layer(layer_name, Some(&base_image)).await?
            } else {
                debug!("Language layer {} already exists", lang_image_name);
                lang_image_name.to_string()
            }
        }
    };

    info!("Language layer ready: {}", language_image);

    // Step 3: Build final project-specific image
    if let Some(agent) = agent_name {
        // For agents: build base → agent (project-specific)
        // Node.js is now in base, so agents inherit it directly
        info!("Building agent layer for project...");

        // Build project-specific agent image
        let agent_layer = format!("agent-{}", agent);
        let final_image_name = get_agent_project_image_name(agent, &project_hash);

        if force_rebuild || !image_exists(&final_image_name).await? {
            info!(
                "Building project-specific agent image: {}",
                final_image_name
            );
            build_image_from_containerfile(&agent_layer, Some(&base_image), &final_image_name)
                .await?;
        } else {
            debug!("Project-specific agent image already exists");
        }

        info!("Final image: {}", final_image_name);
        Ok(final_image_name)
    } else {
        // No agent: just tag language image with project hash
        let layer_type = project_type.language_layer();
        let final_image_name = get_project_image_name(layer_type, &project_hash);

        if force_rebuild || !image_exists(&final_image_name).await? {
            info!("Tagging language image for project: {}", final_image_name);

            let mut cmd = Command::new("podman");
            cmd.arg("tag")
                .arg(&language_image)
                .arg(&final_image_name);

            let status = cmd.status().await.map_err(|e| {
                JailError::Backend(format!("Failed to tag image: {}", e))
            })?;

            if !status.success() {
                return Err(JailError::Backend(format!(
                    "Failed to tag image {} as {}",
                    language_image, final_image_name
                )));
            }

            info!("Tagged {} as {}", language_image, final_image_name);
        } else {
            debug!("Project-specific image already exists");
        }

        info!("Final image: {}", final_image_name);
        Ok(final_image_name)
    }
}

/// Ensure the appropriate image is available for the workspace and agent
pub async fn ensure_layered_image_available(
    workspace_path: &Path,
    agent_name: Option<&str>,
    force_rebuild: bool,
) -> Result<String> {
    build_project_image(workspace_path, agent_name, force_rebuild).await
}

#[cfg(test)]
mod tests {
    use super::*;

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

        // Hash should be 8 characters
        assert_eq!(hash1.len(), 8);

        // Same path should generate same hash
        let hash2 = generate_project_hash(&path1);
        assert_eq!(hash1, hash2);

        // Different path should generate different hash
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
        assert!(get_containerfile_content("agent-claude").is_some());
        assert!(get_containerfile_content("unknown").is_none());
    }
}
