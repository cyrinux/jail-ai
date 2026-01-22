use crate::error::{JailError, Result};
use crate::project_detection::{
    detect_project_type_with_options, has_custom_containerfile, ProjectType,
    CUSTOM_CONTAINERFILE_NAME,
};
use indicatif::{ProgressBar, ProgressStyle};
use lru::LruCache;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
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
const AWS_IMAGE_NAME: &str = "localhost/jail-ai-aws:latest";
const GCP_IMAGE_NAME: &str = "localhost/jail-ai-gcp:latest";

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
const AWS_CONTAINERFILE: &str = include_str!("../containerfiles/aws.Containerfile");
const GCP_CONTAINERFILE: &str = include_str!("../containerfiles/gcp.Containerfile");
const AGENT_CLAUDE_CONTAINERFILE: &str =
    include_str!("../containerfiles/agent-claude.Containerfile");
const AGENT_CLAUDE_CODE_ROUTER_CONTAINERFILE: &str =
    include_str!("../containerfiles/agent-claude-code-router.Containerfile");
const AGENT_COPILOT_CONTAINERFILE: &str =
    include_str!("../containerfiles/agent-copilot.Containerfile");
const AGENT_CURSOR_CONTAINERFILE: &str =
    include_str!("../containerfiles/agent-cursor.Containerfile");
const AGENT_GEMINI_CONTAINERFILE: &str =
    include_str!("../containerfiles/agent-gemini.Containerfile");
const AGENT_CODEX_CONTAINERFILE: &str = include_str!("../containerfiles/agent-codex.Containerfile");
const AGENT_JULES_CONTAINERFILE: &str = include_str!("../containerfiles/agent-jules.Containerfile");

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
        "aws" => "☁️",
        "gcp" => "🌐",
        "custom" => "🎨",
        "agent-claude-code-router" => "🔀",
        "agent-claude" => "🤖",
        "agent-copilot" => "🦾",
        "agent-cursor" => "➡️",
        "agent-gemini" => "🔮",
        "agent-codex" => "💻",
        "agent-jules" => "🚀",
        _ => "📦",
    }
}

// ========== Performance Optimization: Project Hash Memoization ==========

/// Global cache for project hashes
/// This prevents repeated canonicalize + SHA256 calculations for the same workspace
static PROJECT_HASH_CACHE: OnceLock<Arc<Mutex<HashMap<PathBuf, String>>>> = OnceLock::new();

fn project_hash_cache() -> &'static Arc<Mutex<HashMap<PathBuf, String>>> {
    PROJECT_HASH_CACHE.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

/// Generate a project identifier hash from workspace path (with memoization)
///
/// Performance optimization: Caches hash calculations to avoid repeated
/// canonicalize() and SHA256 operations for the same workspace.
fn generate_project_hash(workspace_path: &Path) -> String {
    let abs_path = workspace_path
        .canonicalize()
        .unwrap_or_else(|_| workspace_path.to_path_buf());

    // Check cache first
    {
        let cache = project_hash_cache();
        if let Ok(cache_guard) = cache.lock() {
            if let Some(hash) = cache_guard.get(&abs_path) {
                debug!("✅ Cache hit for project hash: {}", abs_path.display());
                return hash.clone();
            }
        }
    }

    // Cache miss: calculate hash
    debug!(
        "🔍 Cache miss, calculating project hash: {}",
        abs_path.display()
    );
    let mut hasher = Sha256::new();
    hasher.update(abs_path.to_string_lossy().as_bytes());
    let hash = hasher.finalize();
    let hash_hex = hex::encode(hash);
    let short_hash = hash_hex[..8].to_string();

    // Store in cache
    {
        let cache = project_hash_cache();
        if let Ok(mut cache_guard) = cache.lock() {
            cache_guard.insert(abs_path.clone(), short_hash.clone());
        }
    }

    short_hash
}

/// Generate a layer-based tag from project type, custom layer, and agent
/// Format: base-{lang1}-{lang2}-custom-{agent} or base-{lang1}-{lang2} (no agent)
/// Examples: "base-rust-nodejs-custom-claude", "base-python-custom", "base-custom", "base"
fn generate_layer_tag(
    project_type: &ProjectType,
    has_custom: bool,
    agent_name: Option<&str>,
) -> String {
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

    // Add custom layer if present
    if has_custom {
        layers.push("custom");
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
        ProjectType::Aws => AWS_IMAGE_NAME,
        ProjectType::Gcp => GCP_IMAGE_NAME,
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
        "aws" => Some(AWS_CONTAINERFILE),
        "gcp" => Some(GCP_CONTAINERFILE),
        "agent-claude-code-router" => Some(AGENT_CLAUDE_CODE_ROUTER_CONTAINERFILE),
        "agent-claude" => Some(AGENT_CLAUDE_CONTAINERFILE),
        "agent-copilot" => Some(AGENT_COPILOT_CONTAINERFILE),
        "agent-cursor" => Some(AGENT_CURSOR_CONTAINERFILE),
        "agent-gemini" => Some(AGENT_GEMINI_CONTAINERFILE),
        "agent-codex" => Some(AGENT_CODEX_CONTAINERFILE),
        "agent-jules" => Some(AGENT_JULES_CONTAINERFILE),
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

// ========== Performance Optimization: Image Existence Cache ==========

/// Global LRU cache for image existence checks
/// This prevents repeated `podman image exists` calls for the same image
static IMAGE_EXISTS_CACHE: OnceLock<Arc<Mutex<LruCache<String, bool>>>> = OnceLock::new();

fn image_cache() -> &'static Arc<Mutex<LruCache<String, bool>>> {
    IMAGE_EXISTS_CACHE.get_or_init(|| {
        // Cache up to 1000 images (more than enough for typical usage)
        Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(1000).unwrap())))
    })
}

/// Invalidate cache entry for an image (call after building/removing an image)
fn invalidate_image_cache(image_name: &str) {
    let cache = image_cache();
    if let Ok(mut cache_guard) = cache.lock() {
        cache_guard.pop(image_name);
        debug!("🗑️  Invalidated cache for image: {}", image_name);
    }
}

/// Check if an image exists locally (with LRU caching)
///
/// Performance optimization: Caches results to avoid repeated `podman image exists` calls.
/// Cache is automatically invalidated when images are built or removed.
pub async fn image_exists(image_name: &str) -> Result<bool> {
    // Check cache first
    {
        let cache = image_cache();
        if let Ok(mut cache_guard) = cache.lock() {
            if let Some(&exists) = cache_guard.get(image_name) {
                debug!("✅ Cache hit for image existence: {}", image_name);
                return Ok(exists);
            }
        }
    }

    // Cache miss: query podman
    debug!("🔍 Cache miss, checking image existence: {}", image_name);
    let mut cmd = Command::new("podman");
    cmd.arg("image").arg("exists").arg(image_name);

    let exists = match cmd.output().await {
        Ok(output) => output.status.success(),
        Err(_) => false,
    };

    // Update cache
    {
        let cache = image_cache();
        if let Ok(mut cache_guard) = cache.lock() {
            cache_guard.put(image_name.to_string(), exists);
        }
    }

    Ok(exists)
}

/// Get the expected image name for a workspace and agent
/// This determines what image should be used based on current project state
/// without actually building it
pub async fn get_expected_image_name(
    workspace_path: &Path,
    agent_name: Option<&str>,
    isolated: bool,
    no_nix: bool,
) -> Result<String> {
    let project_hash = generate_project_hash(workspace_path);
    let project_type = detect_project_type_with_options(workspace_path, no_nix);
    let has_custom = has_custom_containerfile(workspace_path);

    if let Some(agent) = agent_name {
        // Determine the final image tag based on isolation mode
        let image_tag = if isolated {
            // Isolated mode: Use workspace hash
            project_hash
        } else {
            // Shared mode: Use layer composition
            generate_layer_tag(&project_type, has_custom, Some(agent))
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

// ========== Performance Optimization: Batch Image Checks ==========

/// Check multiple images for rebuild in a single podman inspect call
///
/// Performance optimization: Groups multiple image inspections into a single
/// podman command, reducing syscall overhead significantly.
///
/// Returns a Vec of bools indicating which images need rebuild (same order as input)
async fn batch_check_images_need_rebuild(
    images: &[(String, String)], // (image_name, layer_name)
) -> Result<Vec<bool>> {
    if images.is_empty() {
        return Ok(vec![]);
    }

    debug!("🔍 Batch checking {} images for rebuild", images.len());

    // First check which images exist (using our cached function)
    let mut needs_rebuild = Vec::new();
    let mut existing_images = Vec::new();
    let mut existing_indices = Vec::new();

    for (idx, (image_name, layer_name)) in images.iter().enumerate() {
        if !image_exists(image_name).await? {
            debug!("Image {} doesn't exist, needs rebuild", image_name);
            needs_rebuild.push(true);
        } else {
            needs_rebuild.push(false); // Placeholder, will update if needed
            existing_images.push((image_name.clone(), layer_name.clone()));
            existing_indices.push(idx);
        }
    }

    // If no existing images, all need rebuild
    if existing_images.is_empty() {
        return Ok(needs_rebuild);
    }

    // Build single podman inspect command for all existing images
    let image_names: Vec<&str> = existing_images
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();

    let mut cmd = Command::new("podman");
    cmd.arg("image")
        .arg("inspect")
        .arg("--format")
        .arg("{{.Id}}\t{{index .Labels \"ai.jail.containerfile.hash\"}}");

    for name in &image_names {
        cmd.arg(name);
    }

    debug!("Running batch inspect: {:?}", cmd);

    let output = cmd.output().await;

    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = stdout.lines().collect();

            // Parse results and check hashes
            for (i, line) in lines.iter().enumerate() {
                if i >= existing_images.len() {
                    break;
                }

                let (_, layer_name) = &existing_images[i];
                let idx = existing_indices[i];

                let parts: Vec<&str> = line.split('\t').collect();
                let image_hash = if parts.len() > 1 {
                    let hash = parts[1].trim();
                    if hash.is_empty() || hash == "<no value>" {
                        None
                    } else {
                        Some(hash.to_string())
                    }
                } else {
                    None
                };

                // Get current Containerfile hash
                let current_hash = get_containerfile_content(layer_name).map(hash_containerfile);

                // Determine if rebuild is needed
                let rebuild = match (image_hash, current_hash) {
                    (Some(img_hash), Some(cur_hash)) => {
                        let needs = img_hash != cur_hash;
                        if needs {
                            debug!(
                                "Layer {} hash mismatch: image={} current={}",
                                layer_name, img_hash, cur_hash
                            );
                        }
                        needs
                    }
                    (None, Some(_)) => {
                        debug!("Layer {} missing hash label, needs rebuild", layer_name);
                        true
                    }
                    (_, None) => {
                        debug!("Layer {} has no Containerfile, skip rebuild", layer_name);
                        false
                    }
                };

                needs_rebuild[idx] = rebuild;
            }
        }
        _ => {
            // If batch inspect fails, mark all existing images as needing rebuild
            debug!("Batch inspect failed, marking all as needing rebuild");
            for &idx in &existing_indices {
                needs_rebuild[idx] = true;
            }
        }
    }

    Ok(needs_rebuild)
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

/// Check if any layers need rebuilding for a given workspace and agent
/// Returns a list of outdated layer names
///
/// Performance optimization: Uses batch checking to inspect all images in a single call
pub async fn check_layers_need_rebuild(
    workspace_path: &Path,
    agent_name: Option<&str>,
    no_nix: bool,
) -> Result<Vec<String>> {
    let project_type = detect_project_type_with_options(workspace_path, no_nix);
    let has_custom = has_custom_containerfile(workspace_path);

    // Build list of all images to check
    let mut images_to_check: Vec<(String, String)> = Vec::new();

    // Add base layer
    images_to_check.push((BASE_IMAGE_NAME.to_string(), "base".to_string()));

    // Add language layers based on project type
    match &project_type {
        ProjectType::Generic => {
            // Only base layer needed
        }
        ProjectType::Multi(types) => {
            for lang_type in types {
                let layer_name = lang_type.language_layer();
                let image_name = get_language_image_name(lang_type);
                images_to_check.push((image_name.to_string(), layer_name.to_string()));
            }
        }
        _ => {
            let layer_name = project_type.language_layer();
            let image_name = get_language_image_name(&project_type);
            images_to_check.push((image_name.to_string(), layer_name.to_string()));
        }
    }

    // Add agent layer if specified
    if let Some(agent_str) = agent_name {
        // Try to parse agent - if recognized, use proper layer name
        let agent_layer = if let Some(agent) = crate::agents::Agent::from_str(agent_str) {
            agent.layer_name()
        } else {
            format!("agent-{}", agent_str)
        };

        let agent_containerfile = get_containerfile_content(&agent_layer);

        if agent_containerfile.is_some() {
            // We need to check the actual agent image that would be used
            // For shared mode, we use layer-based tagging
            let layer_tag = generate_layer_tag(&project_type, has_custom, Some(agent_str));
            let agent_image = get_agent_project_image_name(agent_str, &layer_tag);
            images_to_check.push((agent_image, agent_layer));
        }
    }

    // 🚀 Batch check all images at once
    let needs_rebuild = batch_check_images_need_rebuild(&images_to_check).await?;

    // Collect outdated layers
    let outdated_layers: Vec<String> = images_to_check
        .iter()
        .zip(needs_rebuild.iter())
        .filter_map(|((_, layer_name), &rebuild)| {
            if rebuild {
                Some(layer_name.clone())
            } else {
                None
            }
        })
        .collect();

    if !outdated_layers.is_empty() {
        debug!("📦 Outdated layers: {:?}", outdated_layers);
    } else {
        debug!("✅ All layers are up to date");
    }

    Ok(outdated_layers)
}

/// Build a custom layer from project's jail-ai.Containerfile
async fn build_custom_layer(
    workspace_path: &Path,
    base_image: &str,
    image_tag: &str,
    verbose: bool,
    no_cache: bool,
) -> Result<String> {
    let custom_containerfile_path = workspace_path.join(CUSTOM_CONTAINERFILE_NAME);

    if !custom_containerfile_path.exists() {
        return Err(JailError::Backend(format!(
            "Custom Containerfile not found: {}",
            custom_containerfile_path.display()
        )));
    }

    // Create spinner if not in verbose mode
    let spinner = if !verbose {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        let emoji = get_layer_emoji("custom");
        pb.set_message(format!("{} Building custom layer...", emoji));
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(pb)
    } else {
        info!("Building custom image: {}", image_tag);
        None
    };

    // Read custom Containerfile content for hashing
    let containerfile_content = tokio::fs::read_to_string(&custom_containerfile_path)
        .await
        .map_err(|e| JailError::Backend(format!("Failed to read custom Containerfile: {}", e)))?;

    // Generate hash of Containerfile content
    let containerfile_hash = hash_containerfile(&containerfile_content);

    // Build command
    let mut cmd = Command::new("podman");
    cmd.arg("build").arg("-t").arg(image_tag);

    if no_cache {
        cmd.arg("--no-cache");
    }

    // Add hash label to track Containerfile changes
    cmd.arg("--label")
        .arg(format!("ai.jail.containerfile.hash={}", containerfile_hash));

    // Add base image build arg
    cmd.arg("--build-arg")
        .arg(format!("BASE_IMAGE={}", base_image));

    // Use the custom Containerfile from workspace
    cmd.arg("-f")
        .arg(&custom_containerfile_path)
        .arg(workspace_path);

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
        let emoji = get_layer_emoji("custom");
        pb.finish_with_message(format!("✓ {} Built custom layer", emoji));
    }

    if !status.success() {
        return Err(JailError::Backend(format!(
            "Failed to build custom layer, build command exited with status: {}",
            status
        )));
    }

    // Invalidate cache after successful build
    invalidate_image_cache(image_tag);

    info!("Successfully built custom layer: {}", image_tag);
    Ok(image_tag.to_string())
}

/// Build a shared layer image (with :latest tag)
///
/// This function is public to allow parallel building from image_parallel module
pub async fn build_shared_layer(
    layer_name: &str,
    base_image: Option<&str>,
    verbose: bool,
    force_rebuild: bool,
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
        "aws" => AWS_IMAGE_NAME.to_string(),
        "gcp" => GCP_IMAGE_NAME.to_string(),
        _ => {
            return Err(JailError::Backend(format!(
                "Unknown shared layer: {}",
                layer_name
            )))
        }
    };

    // Check if image needs to be rebuilt (doesn't exist or Containerfile changed)
    if !force_rebuild && !image_needs_rebuild(&image_name, layer_name).await? {
        debug!("Shared layer {} is up to date", image_name);
        return Ok(image_name);
    }

    build_image_from_containerfile(layer_name, base_image, &image_name, verbose, force_rebuild)
        .await
}

/// Internal function to build an image from a Containerfile
async fn build_image_from_containerfile(
    layer_name: &str,
    base_image: Option<&str>,
    image_tag: &str,
    verbose: bool,
    no_cache: bool,
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

    if no_cache {
        cmd.arg("--no-cache");
    }

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

    // Invalidate cache after successful build
    invalidate_image_cache(image_tag);

    info!("Successfully built: {}", image_tag);
    Ok(image_tag.to_string())
}

/// Build the complete image stack for a project
pub async fn build_project_image(
    workspace_path: &Path,
    agent_name: Option<&str>,
    upgrade: bool,
    force_layers: &[String],
    isolated: bool,
    verbose: bool,
    no_nix: bool,
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
                "aws" => lang_types.push(ProjectType::Aws),
                "gcp" => lang_types.push(ProjectType::Gcp),
                "base" => {}                                         // base is implicit
                "custom" => {}                                       // custom is handled separately
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
        let detected = detect_project_type_with_options(workspace_path, no_nix);
        info!("Detected project type: {:?}", detected);
        detected
    };

    // Step 1: Build base layer (shared :latest)
    let force_base = upgrade || force_layers.contains(&"base".to_string());
    let should_rebuild_base = force_base || !image_exists(BASE_IMAGE_NAME).await?;

    let base_image = if should_rebuild_base {
        if verbose {
            info!("Building base layer...");
        }
        build_shared_layer("base", None, verbose, force_base).await?
    } else {
        debug!("Base layer already exists");
        BASE_IMAGE_NAME.to_string()
    };

    // Step 2: Build language layer (shared :latest) if not generic
    let language_image = match project_type {
        ProjectType::Generic => base_image.clone(),
        ProjectType::Multi(ref types) => {
            // 🚀 Feature: Parallel building for multi-language projects
            // Enable with: JAIL_AI_PARALLEL_BUILD=1
            let parallel_enabled = std::env::var("JAIL_AI_PARALLEL_BUILD")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false);

            if parallel_enabled && types.len() > 1 {
                info!(
                    "🚀 Parallel build enabled for {} language layers",
                    types.len()
                );

                // Use parallel building
                let results = crate::image_parallel::build_language_layers_parallel(
                    &base_image,
                    types,
                    force_layers,
                    upgrade,
                    verbose,
                )
                .await?;

                // Return the last built image (any of them works since they all depend on base)
                results
                    .values()
                    .last()
                    .cloned()
                    .unwrap_or(base_image.clone())
            } else {
                // Sequential building (default, safer)
                if parallel_enabled {
                    debug!("Parallel build skipped: only {} layer(s)", types.len());
                }

                let mut current_image = base_image.clone();
                for lang_type in types {
                    let layer_name = lang_type.language_layer();
                    let lang_image_name = get_language_image_name(lang_type);
                    let should_force_lang =
                        upgrade || force_layers.contains(&layer_name.to_string());
                    let should_rebuild_lang =
                        should_force_lang || !image_exists(lang_image_name).await?;

                    current_image = if should_rebuild_lang {
                        build_shared_layer(
                            layer_name,
                            Some(&current_image),
                            verbose,
                            should_force_lang,
                        )
                        .await?
                    } else {
                        lang_image_name.to_string()
                    };
                }
                current_image
            }
        }
        _ => {
            let layer_name = project_type.language_layer();
            let lang_image_name = get_language_image_name(&project_type);
            let should_force_lang = upgrade || force_layers.contains(&layer_name.to_string());
            let should_rebuild_lang = should_force_lang || !image_exists(lang_image_name).await?;

            if should_rebuild_lang {
                build_shared_layer(layer_name, Some(&base_image), verbose, should_force_lang)
                    .await?
            } else {
                debug!("Language layer {} already exists", lang_image_name);
                lang_image_name.to_string()
            }
        }
    };

    info!("Language layer ready: {}", language_image);

    // Step 2.5: Build custom layer if present
    let has_custom = has_custom_containerfile(workspace_path);
    let custom_image = if has_custom {
        // Generate image name for custom layer
        let custom_layer_tag = if isolated {
            format!("{}-custom", project_hash)
        } else {
            generate_layer_tag(&project_type, true, None)
        };

        let custom_image_name = format!("localhost/jail-ai-custom:{}", custom_layer_tag);

        let should_force_custom = upgrade || force_layers.contains(&"custom".to_string());
        let should_rebuild_custom =
            should_force_custom || !image_exists(&custom_image_name).await?;

        if should_rebuild_custom {
            if verbose {
                info!("Building custom layer: {}", custom_image_name);
            }
            build_custom_layer(
                workspace_path,
                &language_image,
                &custom_image_name,
                verbose,
                should_force_custom,
            )
            .await?
        } else {
            debug!("Custom layer already exists: {}", custom_image_name);
            custom_image_name
        }
    } else {
        language_image.clone()
    };

    info!("Custom layer ready: {}", custom_image);

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
            let layer_tag = generate_layer_tag(&project_type, has_custom, Some(agent));
            info!("Using shared mode: layer-based image ({})", layer_tag);
            layer_tag
        };

        let final_image_name = get_agent_project_image_name(agent, &image_tag);
        let should_force_agent = upgrade || force_layers.contains(&agent_layer);
        let should_rebuild_agent = should_force_agent || !image_exists(&final_image_name).await?;

        if should_rebuild_agent {
            if verbose {
                info!("Building agent image: {}", final_image_name);
            }
            build_image_from_containerfile(
                &agent_layer,
                Some(&custom_image),
                &final_image_name,
                verbose,
                should_force_agent,
            )
            .await?;
        } else {
            debug!("Agent image already exists: {}", final_image_name);
        }

        info!("Final image: {}", final_image_name);
        Ok(final_image_name)
    } else {
        // No agent: just tag custom/language image
        let layer_type = project_type.language_layer();

        // Determine the final image tag based on isolation mode
        let image_tag = if isolated {
            // Isolated mode: Use workspace hash
            info!("Using isolated mode: workspace-specific image");
            project_hash.clone()
        } else {
            // Shared mode: Use layer composition
            let layer_tag = generate_layer_tag(&project_type, has_custom, None);
            info!("Using shared mode: layer-based image ({})", layer_tag);
            layer_tag
        };

        let final_image_name = get_project_image_name(layer_type, &image_tag);

        if upgrade || !image_exists(&final_image_name).await? {
            info!("Tagging custom/language image: {}", final_image_name);

            let mut cmd = Command::new("podman");
            cmd.arg("tag").arg(&custom_image).arg(&final_image_name);

            let status = cmd
                .status()
                .await
                .map_err(|e| JailError::Backend(format!("Failed to tag image: {}", e)))?;

            if !status.success() {
                return Err(JailError::Backend(format!(
                    "Failed to tag image {} as {}",
                    custom_image, final_image_name
                )));
            }

            info!("Tagged {} as {}", custom_image, final_image_name);
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
    upgrade: bool,
    force_layers: &[String],
    isolated: bool,
    verbose: bool,
    no_nix: bool,
) -> Result<String> {
    build_project_image(
        workspace_path,
        agent_name,
        upgrade,
        force_layers,
        isolated,
        verbose,
        no_nix,
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
        assert!(get_containerfile_content("aws").is_some());
        assert!(get_containerfile_content("gcp").is_some());
        assert!(get_containerfile_content("agent-claude").is_some());
        assert!(get_containerfile_content("unknown").is_none());
    }
}
