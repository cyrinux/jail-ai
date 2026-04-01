#[cfg(target_os = "linux")]
mod host_ips;
#[cfg(target_os = "linux")]
mod loader_client;

use crate::error::Result;
use std::net::IpAddr;
use tracing::{debug, info};

#[cfg(target_os = "linux")]
pub use host_ips::get_host_ips;
#[cfg(target_os = "linux")]
use loader_client::load_ebpf_via_helper;

#[cfg(not(target_os = "linux"))]
pub fn get_host_ips() -> Result<Vec<IpAddr>> {
    Ok(Vec::new())
}

#[cfg(target_os = "linux")]
pub struct EbpfHostBlocker {
    link_ids: Vec<u64>,
}

#[cfg(target_os = "linux")]
impl EbpfHostBlocker {
    /// Create a new eBPF host blocker instance
    pub fn new() -> Self {
        Self {
            link_ids: Vec::new(),
        }
    }

    /// Attach eBPF program to container's cgroup to block host IPs
    ///
    /// # Arguments
    /// * `cgroup_path` - Path to the container's cgroup (e.g., "/sys/fs/cgroup/user.slice/...")
    /// * `blocked_ips` - List of IP addresses to block (typically host IPs)
    ///
    /// # Returns
    /// Ok(()) if successful, Err if eBPF loading fails
    ///
    /// # Behavior
    /// - Delegates eBPF loading to jail-ai-ebpf-loader privileged helper binary
    /// - Helper binary loads program, populates BPF maps, and attaches to cgroup
    /// - Helper binary validates inputs and drops capabilities after loading
    /// - Handles IPv4, IPv6, TCP, UDP, ICMP, and all other packet types
    ///
    /// # Errors
    /// - If jail-ai-ebpf-loader binary is not found or lacks capabilities
    /// - If BPF program cannot be loaded
    /// - If BPF maps cannot be populated
    /// - If program cannot be attached to cgroup
    ///
    /// # Security
    /// The main jail-ai binary **does not** require elevated privileges.
    /// Only the helper binary needs CAP_BPF and CAP_NET_ADMIN.
    pub async fn attach_to_cgroup(
        &mut self,
        cgroup_path: &str,
        blocked_ips: &[IpAddr],
    ) -> Result<()> {
        info!(
            "eBPF host blocker: delegating to helper binary for cgroup {} with {} blocked IPs",
            cgroup_path,
            blocked_ips.len()
        );

        // Extract container name from cgroup path
        // Format: /sys/fs/cgroup/.../libpod-CONTAINER_NAME.scope/...
        let container_name = cgroup_path
            .split('/')
            .find(|s| s.starts_with("libpod-") && s.ends_with(".scope"))
            .and_then(|s| s.strip_prefix("libpod-"))
            .and_then(|s| s.strip_suffix(".scope"))
            .unwrap_or("unknown");

        debug!(
            "Extracted container name: {} from cgroup path: {}",
            container_name, cgroup_path
        );

        // Call the helper binary to do the privileged work
        match load_ebpf_via_helper(container_name, cgroup_path, blocked_ips).await {
            Ok(link_ids) => {
                self.link_ids = link_ids;
                info!("✓ eBPF host blocking active for cgroup {}", cgroup_path);
                Ok(())
            }
            Err(e) => {
                let error_msg = e.to_string();
                // If loader is already running, that's fine - don't fail
                if error_msg.contains("Loader already running") {
                    debug!("eBPF loader already running for this container");
                    return Ok(());
                }

                // Return error immediately - don't warn since we're going to crash
                // The error message will be displayed when the application exits
                Err(e)
            }
        }
    }

    /// Detach eBPF program from cgroup
    ///
    /// # Note
    /// eBPF programs are managed by the kernel and will be automatically
    /// detached when the container/cgroup is destroyed. This method is
    /// kept for API compatibility but is currently a no-op.
    #[cfg(test)]
    pub async fn detach(&mut self) -> Result<()> {
        info!("eBPF programs will be automatically detached when container stops");
        self.link_ids.clear();
        Ok(())
    }

    /// Check if eBPF program is currently loaded
    #[cfg(test)]
    pub fn is_loaded(&self) -> bool {
        !self.link_ids.is_empty()
    }
}

#[cfg(target_os = "linux")]
impl Default for EbpfHostBlocker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "linux")]
impl Drop for EbpfHostBlocker {
    fn drop(&mut self) {
        // eBPF programs are managed by the kernel and will be automatically
        // detached when the container/cgroup is destroyed
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ebpf_blocker_creation() {
        let blocker = EbpfHostBlocker::new();
        assert!(!blocker.is_loaded());
    }

    #[tokio::test]
    async fn test_ebpf_blocker_detach() {
        let mut blocker = EbpfHostBlocker::new();
        let result = blocker.detach().await;
        assert!(result.is_ok());
    }
}
