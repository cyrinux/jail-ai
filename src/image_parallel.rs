/// Parallel image building and pre-fetching for multi-language projects
///
/// This module provides:
/// 1. Optimized parallel building of independent language layers
/// 2. Background pre-fetching of commonly needed layers
///
/// For projects with multiple language stacks (e.g., Rust + Node.js + Python),
/// layers can be built concurrently instead of sequentially.
use crate::error::Result;
use crate::image_layers::{build_shared_layer, image_exists};
use crate::project_detection::{detect_project_type_with_options, ProjectType};
use std::collections::HashMap;
use std::path::Path;
use tokio::task::JoinSet;
use tracing::{debug, info};

/// Build multiple independent language layers in parallel
///
/// # Performance
/// For a project with N language layers, this can provide up to N× speedup
/// compared to sequential building, as layers depending only on base can
/// be built concurrently.
///
/// # Arguments
/// * `base_image` - The base image all language layers depend on
/// * `lang_types` - List of language types to build
/// * `force_layers` - Layers to force rebuild (currently unused in parallel mode)
/// * `_upgrade` - Whether to upgrade all layers (currently unused in parallel mode)
/// * `verbose` - Whether to show verbose build output
///
/// # Returns
/// HashMap mapping layer names to their built image names
pub async fn build_language_layers_parallel(
    base_image: &str,
    lang_types: &[ProjectType],
    _force_layers: &[String],
    upgrade: bool,
    verbose: bool,
) -> Result<HashMap<String, String>> {
    if lang_types.is_empty() {
        return Ok(HashMap::new());
    }

    info!(
        "🚀 Building {} language layers in parallel...",
        lang_types.len()
    );

    let mut join_set = JoinSet::new();

    // Spawn parallel builds for each language layer
    for lang_type in lang_types {
        let layer_name = lang_type.language_layer().to_string();
        let base_image = base_image.to_string();

        debug!("Spawning parallel build task for layer: {}", layer_name);

        join_set.spawn(async move {
            let result = build_shared_layer(&layer_name, Some(&base_image), verbose, upgrade).await?;
            Ok::<_, crate::error::JailError>((layer_name, result))
        });
    }

    // Collect results
    let mut results = HashMap::new();
    let mut errors = Vec::new();

    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(Ok((layer_name, image_name))) => {
                debug!(
                    "✓ Parallel build completed: {} -> {}",
                    layer_name, image_name
                );
                results.insert(layer_name, image_name);
            }
            Ok(Err(e)) => {
                errors.push(format!("Build error: {}", e));
            }
            Err(e) => {
                errors.push(format!("Task join error: {}", e));
            }
        }
    }

    // If any builds failed, return error with all failures
    if !errors.is_empty() {
        return Err(crate::error::JailError::Backend(format!(
            "Parallel build failed:\n{}",
            errors.join("\n")
        )));
    }

    info!("✓ Successfully built {} layers in parallel", results.len());

    Ok(results)
}

/// Pre-fetch commonly needed layers in background
///
/// This function spawns a background task to build/ensure layers that are likely
/// to be needed based on the project type. This can significantly reduce perceived
/// latency for subsequent operations.
///
/// # Feature Flag
/// Enable with: JAIL_AI_PREFETCH=1
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
        // Check if pre-fetching is enabled
        let prefetch_enabled = std::env::var("JAIL_AI_PREFETCH")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        if !prefetch_enabled {
            debug!("Pre-fetching disabled (JAIL_AI_PREFETCH not set)");
            return;
        }

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
async fn ensure_layer_exists(layer: &str, base_image: Option<&str>) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_build_empty() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = build_language_layers_parallel("base", &[], &[], false, false).await;
            assert!(result.is_ok());
            assert!(result.unwrap().is_empty());
        });
    }

    #[tokio::test]
    async fn test_ensure_layer_exists_unknown() {
        let result = ensure_layer_exists("unknown-layer", None).await;
        assert!(result.is_ok()); // Should not error on unknown layers
    }
}
