//! Client for communicating with jail-ai-ebpf-loader helper binary

use crate::error::{JailError, Result};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::net::IpAddr;
use std::process::{Command, Stdio};
use tracing::{debug, error, info};

/// Request to load eBPF program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadRequest {
    pub cgroup_path: String,
    pub blocked_ips: Vec<IpAddr>,
}

/// Response from loader
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadResponse {
    pub success: bool,
    pub message: String,
    pub link_ids: Vec<u64>,
}

/// Load eBPF program using the privileged helper binary
///
/// This function:
/// 1. Locates the jail-ai-ebpf-loader binary
/// 2. Sends a LoadRequest via stdin (JSON)
/// 3. Receives a LoadResponse via stdout (JSON)
/// 4. Returns success/failure
///
/// The helper binary requires CAP_BPF and CAP_NET_ADMIN capabilities.
pub async fn load_ebpf_via_helper(
    cgroup_path: &str,
    blocked_ips: &[IpAddr],
) -> Result<Vec<u64>> {
    info!(
        "Loading eBPF program via helper for cgroup: {}",
        cgroup_path
    );

    // Find the helper binary
    let loader_path = find_loader_binary()?;
    debug!("Using loader binary: {}", loader_path.display());

    // Prepare request
    let request = LoadRequest {
        cgroup_path: cgroup_path.to_string(),
        blocked_ips: blocked_ips.to_vec(),
    };

    let request_json = serde_json::to_string(&request).map_err(|e| {
        JailError::Backend(format!("Failed to serialize LoadRequest: {}", e))
    })?;

    debug!("Spawning loader process");
    let mut child = Command::new(&loader_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| {
            JailError::Backend(format!(
                "Failed to spawn loader binary at {}: {}",
                loader_path.display(),
                e
            ))
        })?;

    // Send request to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(request_json.as_bytes()).map_err(|e| {
            JailError::Backend(format!("Failed to write request to loader: {}", e))
        })?;
        stdin.flush().map_err(|e| {
            JailError::Backend(format!("Failed to flush request to loader: {}", e))
        })?;
    } else {
        return Err(JailError::Backend(
            "Failed to get stdin handle for loader".to_string(),
        ));
    }

    // Wait for completion and read response
    let output = child.wait_with_output().map_err(|e| {
        JailError::Backend(format!("Failed to wait for loader process: {}", e))
    })?;

    if !output.status.success() {
        let exit_code = output.status.code().unwrap_or(-1);
        error!("Loader process failed with exit code: {}", exit_code);

        // Try to parse response even on failure
        if let Ok(response_str) = String::from_utf8(output.stdout.clone()) {
            if let Ok(response) = serde_json::from_str::<LoadResponse>(&response_str) {
                return Err(JailError::Backend(format!(
                    "eBPF loader failed: {}",
                    response.message
                )));
            }
        }

        return Err(JailError::Backend(format!(
            "eBPF loader failed with exit code {}",
            exit_code
        )));
    }

    // Parse response
    let response_str = String::from_utf8(output.stdout).map_err(|e| {
        JailError::Backend(format!("Failed to parse loader output as UTF-8: {}", e))
    })?;

    let response: LoadResponse = serde_json::from_str(&response_str).map_err(|e| {
        JailError::Backend(format!(
            "Failed to parse LoadResponse JSON: {}\nOutput: {}",
            e, response_str
        ))
    })?;

    if response.success {
        info!("✓ eBPF program loaded successfully via helper");
        Ok(response.link_ids)
    } else {
        Err(JailError::Backend(format!(
            "eBPF loader failed: {}",
            response.message
        )))
    }
}

/// Find the jail-ai-ebpf-loader binary
///
/// Search order:
/// 1. Same directory as jail-ai binary
/// 2. $PATH
/// 3. /usr/local/bin
/// 4. /usr/bin
fn find_loader_binary() -> Result<std::path::PathBuf> {
    let loader_name = "jail-ai-ebpf-loader";

    // 1. Same directory as current executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let loader_path = exe_dir.join(loader_name);
            if loader_path.exists() {
                debug!("Found loader in exe directory: {:?}", loader_path);
                return Ok(loader_path);
            }
        }
    }

    // 2. Check PATH
    if let Ok(path_result) = which::which(loader_name) {
        debug!("Found loader in PATH: {:?}", path_result);
        return Ok(path_result);
    }

    // 3. Common installation directories
    for dir in &["/usr/local/bin", "/usr/bin"] {
        let loader_path = std::path::Path::new(dir).join(loader_name);
        if loader_path.exists() {
            debug!("Found loader at: {:?}", loader_path);
            return Ok(loader_path);
        }
    }

    // Not found
    Err(JailError::Backend("jail-ai-ebpf-loader not found. Please install it with:\n\
         cargo install --path jail-ai-ebpf-loader\n\
         sudo setcap cap_bpf,cap_net_admin+ep $(which jail-ai-ebpf-loader)".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_loader_binary() {
        // This test will fail if the loader is not installed, which is expected
        match find_loader_binary() {
            Ok(path) => println!("Found loader at: {:?}", path),
            Err(e) => println!("Loader not found (expected in test): {}", e),
        }
    }
}
