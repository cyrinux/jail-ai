/// Parallel image building for multi-language projects
///
/// This module provides optimized parallel building of independent language layers.
/// For projects with multiple language stacks (e.g., Rust + Node.js + Python),
/// layers can be built concurrently instead of sequentially.

use crate::error::Result;
use crate::image_layers::build_shared_layer;
use crate::project_detection::ProjectType;
use std::collections::HashMap;
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
    _upgrade: bool,
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
            let result = build_shared_layer(&layer_name, Some(&base_image), verbose).await?;
            Ok::<_, crate::error::JailError>((layer_name, result))
        });
    }

    // Collect results
    let mut results = HashMap::new();
    let mut errors = Vec::new();

    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(Ok((layer_name, image_name))) => {
                debug!("✓ Parallel build completed: {} -> {}", layer_name, image_name);
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

    info!(
        "✓ Successfully built {} layers in parallel",
        results.len()
    );

    Ok(results)
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
}
