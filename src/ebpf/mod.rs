mod host_ips;
mod loader_client;

use crate::error::Result;
use std::net::IpAddr;
use tracing::{info, warn};

pub use host_ips::get_host_ips;
use loader_client::load_ebpf_via_helper;

/// eBPF-based host blocker for containers
///
/// This struct manages eBPF programs that block all packets from containers to host IPs.
/// It delegates eBPF loading to a privileged helper binary (jail-ai-ebpf-loader).
///
/// # Requirements
/// - jail-ai-ebpf-loader binary must be installed with CAP_BPF and CAP_NET_ADMIN capabilities
/// - Linux kernel 4.10+ with BPF cgroup_skb support
///
/// # Security Architecture
/// - Main jail-ai binary runs **without** elevated privileges
/// - Privileged helper binary (jail-ai-ebpf-loader) performs eBPF loading
/// - Helper binary validates inputs rigorously and drops capabilities after loading
/// - Minimal attack surface: helper is < 500 LOC and stateless
///
/// # Usage
/// ```no_run
/// # use jail_ai::ebpf::{EbpfHostBlocker, get_host_ips};
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut blocker = EbpfHostBlocker::new();
/// let host_ips = get_host_ips()?;
/// blocker.attach_to_cgroup("/sys/fs/cgroup/my-container", &host_ips).await?;
/// # Ok(())
/// # }
/// ```
pub struct EbpfHostBlocker {
    /// Link IDs for cleanup (not currently used but kept for API compatibility)
    _link_ids: Vec<u64>,
}

impl EbpfHostBlocker {
    /// Create a new eBPF host blocker instance
    pub fn new() -> Self {
        Self {
            _link_ids: Vec::new(),
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

        // Call the helper binary to do the privileged work
        match load_ebpf_via_helper(cgroup_path, blocked_ips).await {
            Ok(link_ids) => {
                self._link_ids = link_ids;
                info!("✓ eBPF host blocking active for cgroup {}", cgroup_path);
                Ok(())
            }
            Err(e) => {
                warn!("⚠️  Failed to load eBPF via helper: {}", e);
                warn!("   Host blocking will not be enforced");
                warn!("   To enable eBPF blocking:");
                warn!("   1. Build loader: cargo build --release -p jail-ai-ebpf-loader");
                warn!(
                    "   2. Install loader: cargo install --path jail-ai-ebpf-loader --force"
                );
                warn!("   3. Grant capabilities: sudo setcap cap_bpf,cap_net_admin+ep $(which jail-ai-ebpf-loader)");
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
    #[allow(dead_code)]
    pub async fn detach(&mut self) -> Result<()> {
        info!("eBPF programs will be automatically detached when container stops");
        self._link_ids.clear();
        Ok(())
    }

    /// Check if eBPF program is currently loaded
    #[allow(dead_code)]
    pub fn is_loaded(&self) -> bool {
        !self._link_ids.is_empty()
    }
}

impl Default for EbpfHostBlocker {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for EbpfHostBlocker {
    fn drop(&mut self) {
        // eBPF programs are managed by the kernel and will be automatically
        // detached when the container/cgroup is destroyed
    }
}

#[cfg(test)]
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
