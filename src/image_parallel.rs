/// Parallel image building and pre-fetching for multi-language projects
///
/// This module provides:
/// 1. Sequential building of language layers (base first, then each language)
/// 2. Background pre-fetching of commonly needed layers
///
/// Note: True parallelization isn't possible because each language layer
/// depends on the base image via Dockerfile FROM directive.
use crate::error::Result;
use crate::image_layers::{build_shared_layer, get_language_image_name, image_exists};
use crate::project_detection::{detect_project_type_with_options, ProjectType};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info};

/// Build language layers efficiently - base first, then language layers
///
/// This function ensures:
/// 1. Base image is built once (if needed)
/// 2. Each language layer is built on top of base sequentially
///
/// Note: True parallelization isn't possible because each language layer
/// depends on the base image via Dockerfile FROM directive. We build sequentially
/// but this function provides a clean API for the workflow.
pub async fn build_language_layers_parallel(
    base_image: &str,
    lang_types: &[ProjectType],
    force_layers: &[String],
    upgrade: bool,
    verbose: bool,
) -> Result<HashMap<String, String>> {
    if lang_types.is_empty() {
        return Ok(HashMap::new());
    }

    info!(
        "Building {} language layers on top of {}...",
        lang_types.len(),
        base_image
    );

    // Step 1: Ensure base image exists (build once)
    let base_exists = image_exists(base_image).await?;
    if !base_exists || upgrade || force_layers.contains(&"base".to_string()) {
        info!("Building base layer...");
        build_shared_layer("base", None, verbose, upgrade).await?;
    }

    // Step 2: Build each language layer sequentially (can't parallelize due to FROM dependencies)
    let mut results = HashMap::new();
    let mut current_image = base_image.to_string();

    for lang_type in lang_types {
        let layer_name = lang_type.language_layer().to_string();
        let lang_image_name = get_language_image_name(lang_type);

        let should_force = upgrade || force_layers.contains(&layer_name);
        let needs_build = should_force || !image_exists(lang_image_name).await?;

        if needs_build {
            info!(
                "Building {} layer on top of {}...",
                layer_name, current_image
            );
            current_image =
                build_shared_layer(&layer_name, Some(&current_image), verbose, should_force)
                    .await?;
        } else {
            current_image = lang_image_name.to_string();
        }

        results.insert(layer_name, current_image.clone());
    }

    info!("✓ Successfully built {} language layers", results.len());
    Ok(results)
}

/// Pre-fetch commonly needed layers in background
///
/// This function spawns a background task to build/ensure layers that are likely
/// to be needed based on the project type. This can significantly reduce perceived
/// latency for subsequent operations.
///
/// # Arguments
/// * `workspace_path` - Path to the workspace to analyze
///
/// # Returns
/// A JoinHandle that can be awaited if you want to wait for prefetching to complete,
/// or just dropped to let it run in the background.
pub fn prefetch_common_layers(workspace_path: &Path) -> tokio::task::JoinHandle<()> {
    let workspace_path = workspace_path.to_path_buf();

    tokio::spawn(async move {
        info!("🔮 Starting background pre-fetch of common layers...");

        // Detect project type (fast operation)
        let project_type = detect_project_type_with_options(&workspace_path, false);

        // Ensure base layer exists (most commonly needed)
        if let Err(e) = ensure_layer_exists("base", None).await {
            debug!("Pre-fetch base layer failed: {}", e);
        }

        // Pre-fetch language-specific layers based on detected type
        match project_type {
            ProjectType::Rust => {
                let _ = ensure_layer_exists("rust", Some("base")).await;
            }
            ProjectType::Golang => {
                let _ = ensure_layer_exists("golang", Some("base")).await;
            }
            ProjectType::NodeJS => {
                let _ = ensure_layer_exists("nodejs", Some("base")).await;
            }
            ProjectType::Python => {
                let _ = ensure_layer_exists("python", Some("base")).await;
            }
            ProjectType::Java => {
                let _ = ensure_layer_exists("java", Some("base")).await;
            }
            ProjectType::Nix => {
                let _ = ensure_layer_exists("nix", Some("base")).await;
            }
            ProjectType::Php => {
                let _ = ensure_layer_exists("php", Some("base")).await;
            }
            ProjectType::Cpp => {
                let _ = ensure_layer_exists("cpp", Some("base")).await;
            }
            ProjectType::CSharp => {
                let _ = ensure_layer_exists("csharp", Some("base")).await;
            }
            ProjectType::Multi(types) => {
                // Pre-fetch all detected language layers
                for lang_type in types {
                    let layer = lang_type.language_layer();
                    let _ = ensure_layer_exists(layer, Some("base")).await;
                }
            }
            ProjectType::Generic => {
                // Only base layer needed
            }
            _ => {}
        }

        info!("✓ Background pre-fetch completed");
    })
}

/// Ensure a specific layer exists, building it if necessary
pub async fn ensure_layer_exists(layer: &str, base_image: Option<&str>) -> Result<()> {
    // Get the image name for this layer
    let image_name = match layer {
        "base" => "localhost/jail-ai-base:latest",
        "rust" => "localhost/jail-ai-rust:latest",
        "golang" => "localhost/jail-ai-golang:latest",
        "nodejs" => "localhost/jail-ai-nodejs:latest",
        "python" => "localhost/jail-ai-python:latest",
        "java" => "localhost/jail-ai-java:latest",
        "nix" => "localhost/jail-ai-nix:latest",
        "php" => "localhost/jail-ai-php:latest",
        "cpp" => "localhost/jail-ai-cpp:latest",
        "csharp" => "localhost/jail-ai-csharp:latest",
        _ => {
            debug!("Unknown layer for pre-fetch: {}", layer);
            return Ok(());
        }
    };

    // Check if image already exists (using cached check)
    if image_exists(image_name).await? {
        debug!("✅ Layer {} already exists, skipping pre-fetch", layer);
        return Ok(());
    }

    info!("📥 Pre-fetching layer: {}", layer);

    // Build the layer (non-verbose to avoid cluttering output)
    build_shared_layer(layer, base_image, false, false).await?;

    info!("✓ Pre-fetched layer: {}", layer);
    Ok(())
}
