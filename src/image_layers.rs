use crate::error::{JailError, Result};
use crate::project_detection::{detect_project_type, ProjectType};
use indicatif::{ProgressBar, ProgressStyle};
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
const NIX_IMAGE_NAME: &str = "localhost/jail-ai-nix:latest";
const PHP_IMAGE_NAME: &str = "localhost/jail-ai-php:latest";
const CPP_IMAGE_NAME: &str = "localhost/jail-ai-cpp:latest";
const CSHARP_IMAGE_NAME: &str = "localhost/jail-ai-csharp:latest";
const TERRAFORM_IMAGE_NAME: &str = "localhost/jail-ai-terraform:latest";
const KUBERNETES_IMAGE_NAME: &str = "localhost/jail-ai-kubernetes:latest";

/// Containerfiles embedded from the repository
const BASE_CONTAINERFILE: &str = include_str!("../containerfiles/base.Containerfile");
const GOLANG_CONTAINERFILE: &str = include_str!("../containerfiles/golang.Containerfile");
const RUST_CONTAINERFILE: &str = include_str!("../containerfiles/rust.Containerfile");
const PYTHON_CONTAINERFILE: &str = include_str!("../containerfiles/python.Containerfile");
const NODEJS_CONTAINERFILE: &str = include_str!("../containerfiles/nodejs.Containerfile");
const JAVA_CONTAINERFILE: &str = include_str!("../containerfiles/java.Containerfile");
const NIX_CONTAINERFILE: &str = include_str!("../containerfiles/nix.Containerfile");
const PHP_CONTAINERFILE: &str = include_str!("../containerfiles/php.Containerfile");
const CPP_CONTAINERFILE: &str = include_str!("../containerfiles/cpp.Containerfile");
const CSHARP_CONTAINERFILE: &str = include_str!("../containerfiles/csharp.Containerfile");
const TERRAFORM_CONTAINERFILE: &str = include_str!("../containerfiles/terraform.Containerfile");
const KUBERNETES_CONTAINERFILE: &str = include_str!("../containerfiles/kubernetes.Containerfile");
const AGENT_CLAUDE_CONTAINERFILE: &str =
    include_str!("../containerfiles/agent-claude.Containerfile");
const AGENT_COPILOT_CONTAINERFILE: &str =
    include_str!("../containerfiles/agent-copilot.Containerfile");
const AGENT_CURSOR_CONTAINERFILE: &str =
    include_str!("../containerfiles/agent-cursor.Containerfile");
const AGENT_GEMINI_CONTAINERFILE: &str =
    include_str!("../containerfiles/agent-gemini.Containerfile");
const AGENT_CODEX_CONTAINERFILE: &str = include_str!("../containerfiles/agent-codex.Containerfile");

/// Get emoji for a layer type
fn get_layer_emoji(layer_name: &str) -> &'static str {
    match layer_name {
        "base" => "🏗️",
        "rust" => "🦀",
        "golang" => "🐹",
        "python" => "🐍",
        "nodejs" => "🟢",
        "java" => "☕",
        "nix" => "❄️",
        "php" => "🐘",
        "cpp" => "🔧",
        "csharp" => "🎯",
        "terraform" => "🏗️",
        "kubernetes" => "☸️",
        "agent-claude" => "🤖",
        "agent-copilot" => "🦾",
        "agent-cursor" => "➡️",
        "agent-gemini" => "🔮",
        "agent-codex" => "💻",
        _ => "📦",
    }
}

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

/// Generate a layer-based tag from project type and agent
/// Format: base-{lang1}-{lang2}-{agent} or base-{lang1}-{lang2} (no agent)
/// Examples: "base-rust-nodejs-claude", "base-python", "base"
fn generate_layer_tag(project_type: &ProjectType, agent_name: Option<&str>) -> String {
    let mut layers = vec!["base"];

    match project_type {
        ProjectType::Generic => {
            // Only base layer
        }
        ProjectType::Multi(types) => {
            // Add all language layers
            for lang_type in types {
                layers.push(lang_type.language_layer());
            }
        }
        _ => {
            // Single language
            layers.push(project_type.language_layer());
        }
    }

    // Add agent if present
    if let Some(agent) = agent_name {
        layers.push(agent);
    }

    layers.join("-")
}

/// Get the shared language image name (with :latest tag)
fn get_language_image_name(project_type: &ProjectType) -> &'static str {
    match project_type {
        ProjectType::Rust => RUST_IMAGE_NAME,
        ProjectType::Golang => GOLANG_IMAGE_NAME,
        ProjectType::Python => PYTHON_IMAGE_NAME,
        ProjectType::NodeJS => NODEJS_IMAGE_NAME,
        ProjectType::Java => JAVA_IMAGE_NAME,
        ProjectType::Nix => NIX_IMAGE_NAME,
        ProjectType::Php => PHP_IMAGE_NAME,
        ProjectType::Cpp => CPP_IMAGE_NAME,
        ProjectType::CSharp => CSHARP_IMAGE_NAME,
        ProjectType::Terraform => TERRAFORM_IMAGE_NAME,
        ProjectType::Kubernetes => KUBERNETES_IMAGE_NAME,
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
        "nix" => Some(NIX_CONTAINERFILE),
        "php" => Some(PHP_CONTAINERFILE),
        "cpp" => Some(CPP_CONTAINERFILE),
        "csharp" => Some(CSHARP_CONTAINERFILE),
        "terraform" => Some(TERRAFORM_CONTAINERFILE),
        "kubernetes" => Some(KUBERNETES_CONTAINERFILE),
        "agent-claude" => Some(AGENT_CLAUDE_CONTAINERFILE),
        "agent-copilot" => Some(AGENT_COPILOT_CONTAINERFILE),
        "agent-cursor" => Some(AGENT_CURSOR_CONTAINERFILE),
        "agent-gemini" => Some(AGENT_GEMINI_CONTAINERFILE),
        "agent-codex" => Some(AGENT_CODEX_CONTAINERFILE),
        _ => None,
    }
}

/// Generate a hash of the Containerfile content
fn hash_containerfile(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let hash = hasher.finalize();
    hex::encode(hash)[..16].to_string() // Use first 16 chars
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

/// Get the expected image name for a workspace and agent
/// This determines what image should be used based on current project state
/// without actually building it
pub async fn get_expected_image_name(
    workspace_path: &Path,
    agent_name: Option<&str>,
    isolated: bool,
) -> Result<String> {
    let project_hash = generate_project_hash(workspace_path);
    let project_type = detect_project_type(workspace_path);

    if let Some(agent) = agent_name {
        // Determine the final image tag based on isolation mode
        let image_tag = if isolated {
            // Isolated mode: Use workspace hash
            project_hash
        } else {
            // Shared mode: Use layer composition
            generate_layer_tag(&project_type, Some(agent))
        };

        Ok(get_agent_project_image_name(agent, &image_tag))
    } else {
        // No agent specified, use project image
        let image_tag = if isolated {
            project_hash
        } else {
            project_type.language_layer().to_string()
        };

        Ok(get_project_image_name(
            project_type.language_layer(),
            &image_tag,
        ))
    }
}

/// Get the containerfile hash label from an image
async fn get_image_containerfile_hash(image_name: &str) -> Result<Option<String>> {
    let mut cmd = Command::new("podman");
    cmd.arg("image")
        .arg("inspect")
        .arg(image_name)
        .arg("--format")
        .arg("{{index .Labels \"ai.jail.containerfile.hash\"}}");

    match cmd.output().await {
        Ok(output) if output.status.success() => {
            let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if hash.is_empty() || hash == "<no value>" {
                Ok(None)
            } else {
                Ok(Some(hash))
            }
        }
        _ => Ok(None),
    }
}

/// Check if an image needs to be rebuilt based on Containerfile changes
async fn image_needs_rebuild(image_name: &str, layer_name: &str) -> Result<bool> {
    // If image doesn't exist, it needs to be built
    if !image_exists(image_name).await? {
        return Ok(true);
    }

    // Get the current Containerfile content
    let containerfile_content = match get_containerfile_content(layer_name) {
        Some(content) => content,
        None => return Ok(false), // Unknown layer, don't rebuild
    };

    // Calculate current hash
    let current_hash = hash_containerfile(containerfile_content);

    // Get the hash from the image
    let image_hash = get_image_containerfile_hash(image_name).await?;

    // Rebuild if hashes don't match
    match image_hash {
        Some(hash) => Ok(hash != current_hash),
        None => {
            // No hash label found, rebuild to add it
            debug!(
                "No containerfile hash label found for {}, rebuilding",
                image_name
            );
            Ok(true)
        }
    }
}

/// Build a shared layer image (with :latest tag)
async fn build_shared_layer(
    layer_name: &str,
    base_image: Option<&str>,
    verbose: bool,
) -> Result<String> {
    let image_name = match layer_name {
        "base" => BASE_IMAGE_NAME.to_string(),
        "golang" => GOLANG_IMAGE_NAME.to_string(),
        "rust" => RUST_IMAGE_NAME.to_string(),
        "python" => PYTHON_IMAGE_NAME.to_string(),
        "nodejs" => NODEJS_IMAGE_NAME.to_string(),
        "java" => JAVA_IMAGE_NAME.to_string(),
        "nix" => NIX_IMAGE_NAME.to_string(),
        "php" => PHP_IMAGE_NAME.to_string(),
        "cpp" => CPP_IMAGE_NAME.to_string(),
        "csharp" => CSHARP_IMAGE_NAME.to_string(),
        "terraform" => TERRAFORM_IMAGE_NAME.to_string(),
        "kubernetes" => KUBERNETES_IMAGE_NAME.to_string(),
        _ => {
            return Err(JailError::Backend(format!(
                "Unknown shared layer: {}",
                layer_name
            )))
        }
    };

    // Check if image needs to be rebuilt (doesn't exist or Containerfile changed)
    if !image_needs_rebuild(&image_name, layer_name).await? {
        debug!("Shared layer {} is up to date", image_name);
        return Ok(image_name);
    }

    build_image_from_containerfile(layer_name, base_image, &image_name, verbose).await
}

/// Internal function to build an image from a Containerfile
async fn build_image_from_containerfile(
    layer_name: &str,
    base_image: Option<&str>,
    image_tag: &str,
    verbose: bool,
) -> Result<String> {
    // Create spinner if not in verbose mode
    let spinner = if !verbose {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        let emoji = get_layer_emoji(layer_name);
        pb.set_message(format!("{} Building {} layer...", emoji, layer_name));
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(pb)
    } else {
        info!("Building image: {} -> {}", layer_name, image_tag);
        None
    };

    let containerfile_content = get_containerfile_content(layer_name).ok_or_else(|| {
        JailError::Backend(format!("No Containerfile found for layer: {}", layer_name))
    })?;

    // Generate hash of Containerfile content
    let containerfile_hash = hash_containerfile(containerfile_content);

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

    // Add hash label to track Containerfile changes
    cmd.arg("--label")
        .arg(format!("ai.jail.containerfile.hash={}", containerfile_hash));

    // Add base image build arg if provided
    if let Some(base) = base_image {
        cmd.arg("--build-arg").arg(format!("BASE_IMAGE={}", base));
    }

    cmd.arg("-f").arg(&containerfile_path).arg(temp_dir.path());

    debug!("Running build command: {:?}", cmd);

    use std::process::Stdio;
    let status = if verbose {
        // In verbose mode, show all output
        cmd.stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await
            .map_err(|e| JailError::Backend(format!("Failed to execute build command: {}", e)))?
    } else {
        // In non-verbose mode, hide output
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map_err(|e| JailError::Backend(format!("Failed to execute build command: {}", e)))?
    };

    if let Some(pb) = spinner {
        let emoji = get_layer_emoji(layer_name);
        pb.finish_with_message(format!("✓ {} Built {} layer", emoji, layer_name));
    }

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
    force_layers: &[String],
    isolated: bool,
    verbose: bool,
) -> Result<String> {
    // Generate project-specific identifier (for isolated mode)
    let project_hash = generate_project_hash(workspace_path);
    if isolated {
        info!("Project hash (isolated mode): {}", project_hash);
    }

    // Detect project type (skip autodetection if force_layers is specified)
    let project_type = if !force_layers.is_empty() {
        // Bypass autodetection: use only the specified layers
        info!(
            "Bypassing autodetection: using specified layers: {:?}",
            force_layers
        );

        // Build a synthetic ProjectType from specified layers
        let mut lang_types = Vec::new();
        for layer in force_layers {
            match layer.as_str() {
                "rust" => lang_types.push(ProjectType::Rust),
                "golang" => lang_types.push(ProjectType::Golang),
                "python" => lang_types.push(ProjectType::Python),
                "nodejs" => lang_types.push(ProjectType::NodeJS),
                "java" => lang_types.push(ProjectType::Java),
                "nix" => lang_types.push(ProjectType::Nix),
                "php" => lang_types.push(ProjectType::Php),
                "cpp" => lang_types.push(ProjectType::Cpp),
                "csharp" => lang_types.push(ProjectType::CSharp),
                "terraform" => lang_types.push(ProjectType::Terraform),
                "kubernetes" => lang_types.push(ProjectType::Kubernetes),
                "base" => {}                                         // base is implicit
                layer_name if layer_name.starts_with("agent-") => {} // ignore agent layers
                _ => debug!("Unknown layer '{}' in force_layers, ignoring", layer),
            }
        }

        match lang_types.len() {
            0 => ProjectType::Generic,
            1 => lang_types[0].clone(),
            _ => ProjectType::Multi(lang_types),
        }
    } else {
        // Auto-detect project type
        let detected = detect_project_type(workspace_path);
        info!("Detected project type: {:?}", detected);
        detected
    };

    // Step 1: Build base layer (shared :latest)
    let should_rebuild_base = force_rebuild
        || force_layers.contains(&"base".to_string())
        || !image_exists(BASE_IMAGE_NAME).await?;

    let base_image = if should_rebuild_base {
        if verbose {
            info!("Building base layer...");
        }
        build_shared_layer("base", None, verbose).await?
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
                let should_rebuild_lang = force_rebuild
                    || force_layers.contains(&layer_name.to_string())
                    || !image_exists(lang_image_name).await?;

                current_image = if should_rebuild_lang {
                    build_shared_layer(layer_name, Some(&current_image), verbose).await?
                } else {
                    lang_image_name.to_string()
                };
            }
            current_image
        }
        _ => {
            let layer_name = project_type.language_layer();
            let lang_image_name = get_language_image_name(&project_type);
            let should_rebuild_lang = force_rebuild
                || force_layers.contains(&layer_name.to_string())
                || !image_exists(lang_image_name).await?;

            if should_rebuild_lang {
                build_shared_layer(layer_name, Some(&base_image), verbose).await?
            } else {
                debug!("Language layer {} already exists", lang_image_name);
                lang_image_name.to_string()
            }
        }
    };

    info!("Language layer ready: {}", language_image);

    // Step 3: Build final project-specific or layer-based image
    if let Some(agent) = agent_name {
        // For agents: build base → language layers → agent
        // This ensures agent has all language tooling (rust, nix, etc.)

        let agent_layer = format!("agent-{}", agent);

        // Determine the final image tag based on isolation mode
        let image_tag = if isolated {
            // Isolated mode: Use workspace hash
            info!("Using isolated mode: workspace-specific image");
            project_hash.clone()
        } else {
            // Shared mode: Use layer composition
            let layer_tag = generate_layer_tag(&project_type, Some(agent));
            info!("Using shared mode: layer-based image ({})", layer_tag);
            layer_tag
        };

        let final_image_name = get_agent_project_image_name(agent, &image_tag);
        let should_rebuild_agent = force_rebuild
            || force_layers.contains(&agent_layer)
            || !image_exists(&final_image_name).await?;

        if should_rebuild_agent {
            if verbose {
                info!("Building agent image: {}", final_image_name);
            }
            build_image_from_containerfile(
                &agent_layer,
                Some(&language_image),
                &final_image_name,
                verbose,
            )
            .await?;
        } else {
            debug!("Agent image already exists: {}", final_image_name);
        }

        info!("Final image: {}", final_image_name);
        Ok(final_image_name)
    } else {
        // No agent: just tag language image
        let layer_type = project_type.language_layer();

        // Determine the final image tag based on isolation mode
        let image_tag = if isolated {
            // Isolated mode: Use workspace hash
            info!("Using isolated mode: workspace-specific image");
            project_hash.clone()
        } else {
            // Shared mode: Use layer composition
            let layer_tag = generate_layer_tag(&project_type, None);
            info!("Using shared mode: layer-based image ({})", layer_tag);
            layer_tag
        };

        let final_image_name = get_project_image_name(layer_type, &image_tag);

        if force_rebuild || !image_exists(&final_image_name).await? {
            info!("Tagging language image: {}", final_image_name);

            let mut cmd = Command::new("podman");
            cmd.arg("tag").arg(&language_image).arg(&final_image_name);

            let status = cmd
                .status()
                .await
                .map_err(|e| JailError::Backend(format!("Failed to tag image: {}", e)))?;

            if !status.success() {
                return Err(JailError::Backend(format!(
                    "Failed to tag image {} as {}",
                    language_image, final_image_name
                )));
            }

            info!("Tagged {} as {}", language_image, final_image_name);
        } else {
            debug!("Image already exists: {}", final_image_name);
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
    force_layers: &[String],
    isolated: bool,
    verbose: bool,
) -> Result<String> {
    build_project_image(
        workspace_path,
        agent_name,
        force_rebuild,
        force_layers,
        isolated,
        verbose,
    )
    .await
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
        assert_eq!(get_language_image_name(&ProjectType::Nix), NIX_IMAGE_NAME);
        assert_eq!(get_language_image_name(&ProjectType::Php), PHP_IMAGE_NAME);
        assert_eq!(get_language_image_name(&ProjectType::Cpp), CPP_IMAGE_NAME);
        assert_eq!(
            get_language_image_name(&ProjectType::CSharp),
            CSHARP_IMAGE_NAME
        );
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
        assert!(get_containerfile_content("nix").is_some());
        assert!(get_containerfile_content("php").is_some());
        assert!(get_containerfile_content("cpp").is_some());
        assert!(get_containerfile_content("csharp").is_some());
        assert!(get_containerfile_content("agent-claude").is_some());
        assert!(get_containerfile_content("unknown").is_none());
    }
}
