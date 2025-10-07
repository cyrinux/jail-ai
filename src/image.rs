use crate::cli::DEFAULT_IMAGE;
use crate::error::{JailError, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{debug, info};

/// Default image name for jail-ai
pub const DEFAULT_IMAGE_NAME: &str = DEFAULT_IMAGE;

/// Embedded Containerfile content (from repository)
const EMBEDDED_CONTAINERFILE: &str = include_str!("../Containerfile");

/// Get the config directory path (XDG_CONFIG_HOME or ~/.config/jail-ai)
fn get_config_dir() -> PathBuf {
    let base_dir = if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(config_home)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config")
    } else {
        PathBuf::from("/tmp")
    };

    base_dir.join("jail-ai")
}

/// Get the path to the user's Containerfile
fn get_containerfile_path() -> PathBuf {
    get_config_dir().join("Containerfile")
}

/// Get the path to the Containerfile hash cache
fn get_containerfile_hash_path() -> PathBuf {
    get_config_dir().join(".containerfile.sha256")
}

/// Calculate SHA256 hash of a string
fn calculate_string_hash(content: &str) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();

    format!("{:x}", result)
}

/// Show diff between two text contents with context
fn show_diff(old_content: &str, new_content: &str) {
    use similar::{ChangeTag, TextDiff};
    use std::io::{self, Write};

    // ANSI color codes
    const RED: &str = "\x1b[31m";
    const GREEN: &str = "\x1b[32m";
    const RESET: &str = "\x1b[0m";

    println!("\n--- Current Containerfile (in config)");
    println!("+++ New Containerfile (from project)");

    let diff = TextDiff::from_lines(old_content, new_content);

    for (idx, group) in diff.grouped_ops(5).iter().enumerate() {
        if idx > 0 {
            println!("  ...");
        }

        for op in group {
            for change in diff.iter_changes(op) {
                let prefix = match change.tag() {
                    ChangeTag::Delete => "-",
                    ChangeTag::Insert => "+",
                    ChangeTag::Equal => " ",
                };

                let color = match change.tag() {
                    ChangeTag::Delete => RED,
                    ChangeTag::Insert => GREEN,
                    ChangeTag::Equal => "",
                };

                print!("{}{}{}", color, prefix, change);
                if !change.value().ends_with('\n') {
                    println!();
                }
                print!("{}", RESET);
            }
        }
    }

    println!();
    io::stdout().flush().unwrap();
}

/// Prompt user for yes/no input
fn prompt_user(message: &str) -> bool {
    use std::io::{self, Write};

    print!("{} [y/N]: ", message);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

/// Ensure the config directory exists and contains a Containerfile
async fn ensure_containerfile_exists() -> Result<PathBuf> {
    let config_dir = get_config_dir();
    let containerfile_path = get_containerfile_path();

    // Create config directory if it doesn't exist
    if !config_dir.exists() {
        info!("Creating config directory: {}", config_dir.display());
        tokio::fs::create_dir_all(&config_dir)
            .await
            .map_err(|e| JailError::Backend(format!("Failed to create config directory: {}", e)))?;
    }

    // Calculate embedded Containerfile hash
    let embedded_hash = calculate_string_hash(EMBEDDED_CONTAINERFILE);

    // Check if user's Containerfile exists and differs from embedded one
    if containerfile_path.exists() {
        let user_hash = calculate_file_hash(&containerfile_path).await?;

        if user_hash != embedded_hash {
            debug!("User Containerfile hash ({}) differs from embedded hash ({})", user_hash, embedded_hash);

            // Read user's Containerfile for diff display
            let user_content = tokio::fs::read_to_string(&containerfile_path)
                .await
                .map_err(|e| JailError::Backend(format!("Failed to read user Containerfile: {}", e)))?;

            // Show diff between user's and embedded Containerfile
            show_diff(&user_content, EMBEDDED_CONTAINERFILE);

            // Prompt user to replace
            if prompt_user("Containerfile in project changed, want to replace your own one?") {
                info!("Replacing user Containerfile with updated version");
                tokio::fs::remove_file(&containerfile_path)
                    .await
                    .map_err(|e| JailError::Backend(format!("Failed to remove old Containerfile: {}", e)))?;

                // Also remove the hash cache to trigger rebuild
                let hash_path = get_containerfile_hash_path();
                if hash_path.exists() {
                    tokio::fs::remove_file(&hash_path)
                        .await
                        .map_err(|e| JailError::Backend(format!("Failed to remove hash cache: {}", e)))?;
                }

                // Write new Containerfile
                tokio::fs::write(&containerfile_path, EMBEDDED_CONTAINERFILE)
                    .await
                    .map_err(|e| JailError::Backend(format!("Failed to write Containerfile: {}", e)))?;

                info!("Containerfile updated at {}", containerfile_path.display());
            } else {
                info!("Keeping existing Containerfile");
            }
        } else {
            debug!("User Containerfile matches embedded version");
        }
    } else {
        // Write embedded Containerfile if it doesn't exist
        info!(
            "Writing default Containerfile to {}",
            containerfile_path.display()
        );
        tokio::fs::write(&containerfile_path, EMBEDDED_CONTAINERFILE)
            .await
            .map_err(|e| JailError::Backend(format!("Failed to write Containerfile: {}", e)))?;

        info!(
            "Default Containerfile written. You can customize it at {}",
            containerfile_path.display()
        );
    }

    Ok(containerfile_path)
}

/// Calculate SHA256 hash of a file
async fn calculate_file_hash(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};

    let content = tokio::fs::read(path)
        .await
        .map_err(|e| JailError::Backend(format!("Failed to read file for hashing: {}", e)))?;

    let mut hasher = Sha256::new();
    hasher.update(&content);
    let result = hasher.finalize();

    Ok(format!("{:x}", result))
}

/// Check if the Containerfile has changed since last build
async fn has_containerfile_changed(containerfile_path: &Path) -> Result<bool> {
    let hash_path = get_containerfile_hash_path();

    // If no hash file exists, consider it changed (first build)
    if !hash_path.exists() {
        debug!("No hash file found, image needs to be built");
        return Ok(true);
    }

    // Calculate current hash
    let current_hash = calculate_file_hash(containerfile_path).await?;

    // Read stored hash
    let stored_hash = tokio::fs::read_to_string(&hash_path)
        .await
        .unwrap_or_default()
        .trim()
        .to_string();

    let changed = current_hash != stored_hash;
    if changed {
        debug!(
            "Containerfile has changed (current: {}, stored: {})",
            current_hash, stored_hash
        );
    } else {
        debug!("Containerfile unchanged (hash: {})", current_hash);
    }

    Ok(changed)
}

/// Store the hash of the Containerfile after successful build
async fn store_containerfile_hash(containerfile_path: &Path) -> Result<()> {
    let hash = calculate_file_hash(containerfile_path).await?;
    let hash_path = get_containerfile_hash_path();

    tokio::fs::write(&hash_path, hash.as_bytes())
        .await
        .map_err(|e| JailError::Backend(format!("Failed to store Containerfile hash: {}", e)))?;

    debug!("Stored Containerfile hash: {}", hash);
    Ok(())
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

/// Build the jail-ai image from the Containerfile
async fn build_image_from_containerfile(containerfile_path: &Path, image_name: &str) -> Result<()> {
    info!(
        "Building image {} from {}",
        image_name,
        containerfile_path.display()
    );

    let mut cmd = Command::new("podman");
    cmd.arg("build")
        .arg("-t")
        .arg(image_name)
        .arg("-f")
        .arg(containerfile_path)
        .arg(
            containerfile_path
                .parent()
                .unwrap_or_else(|| Path::new(".")),
        );

    debug!("Running build command: {:?}", cmd);

    // Run with inherited stdio so user can see build progress
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
            "Failed to build image, build command exited with status: {}",
            status
        )));
    }

    info!("Successfully built image: {}", image_name);
    Ok(())
}

/// Ensure the jail-ai image exists, building it if necessary
pub async fn ensure_image_available(image_name: &str, force_rebuild: bool) -> Result<()> {
    // Only manage the default image automatically
    if image_name != DEFAULT_IMAGE_NAME {
        debug!(
            "Using custom image {}, skipping automatic build",
            image_name
        );
        // Still check if it exists and provide helpful error
        if !image_exists(image_name).await? {
            return Err(JailError::Backend(format!(
                "Image '{}' not found. Please pull or build it manually, or use the default image.",
                image_name
            )));
        }
        return Ok(());
    }

    let containerfile_path = ensure_containerfile_exists().await?;
    let containerfile_changed = has_containerfile_changed(&containerfile_path).await?;
    let image_available = image_exists(image_name).await?;

    // Build if image doesn't exist, Containerfile has changed, or force_rebuild is true
    if !image_available {
        info!("Image {} not found locally, building...", image_name);
        build_image_from_containerfile(&containerfile_path, image_name).await?;
        store_containerfile_hash(&containerfile_path).await?;
    } else if force_rebuild {
        info!("Force rebuilding image {}...", image_name);
        build_image_from_containerfile(&containerfile_path, image_name).await?;
        store_containerfile_hash(&containerfile_path).await?;
    } else if containerfile_changed {
        info!(
            "Containerfile has changed, rebuilding image {}...",
            image_name
        );
        build_image_from_containerfile(&containerfile_path, image_name).await?;
        store_containerfile_hash(&containerfile_path).await?;
    } else {
        debug!("Image {} is available and up-to-date", image_name);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::const_is_empty)]
    fn test_embedded_containerfile_not_empty() {
        assert!(!EMBEDDED_CONTAINERFILE.is_empty());
        assert!(EMBEDDED_CONTAINERFILE.contains("FROM"));
    }

    #[test]
    fn test_default_image_name() {
        assert_eq!(DEFAULT_IMAGE_NAME, "localhost/jail-ai-env:latest");
    }

    #[tokio::test]
    async fn test_calculate_hash_consistency() {
        use std::io::Write;
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Write test content
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(b"test content").unwrap();
        drop(file);

        // Calculate hash twice
        let hash1 = calculate_file_hash(&file_path).await.unwrap();
        let hash2 = calculate_file_hash(&file_path).await.unwrap();

        assert_eq!(hash1, hash2);
        assert!(!hash1.is_empty());
    }
}
