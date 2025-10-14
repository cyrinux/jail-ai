mod host_ips;
pub mod monitor;

use crate::error::{JailError, Result};
use aya::{
    maps::HashMap as AyaHashMap,
    programs::{cgroup_skb::CgroupSkbLinkId, CgroupSkb, CgroupSkbAttachType},
    Bpf,
};

#[cfg(not(debug_assertions))]
use aya::include_bytes_aligned;
use std::fs::File;
use std::net::{IpAddr, Ipv6Addr};
use tracing::{debug, info};

#[cfg(debug_assertions)]
use tracing::warn;

pub use host_ips::get_host_ips;
pub use monitor::ExecMonitor;

/// Detect the appropriate eBPF target based on host architecture
/// Only used in debug builds for dynamic path construction
#[cfg(debug_assertions)]
fn detect_ebpf_target() -> &'static str {
    // Use conditional compilation to detect architecture at compile time
    #[cfg(target_arch = "x86_64")]
    {
        "bpfel-unknown-none"
    }
    #[cfg(target_arch = "aarch64")]
    {
        "bpfel-unknown-none"
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        compile_error!("Unsupported architecture for eBPF. Supported: x86_64, aarch64");
    }
}

/// Path to eBPF program in debug mode (dynamically constructed based on architecture)
#[cfg(debug_assertions)]
fn get_ebpf_program_path() -> String {
    format!(
        "{}/target/{}/release/jail-ai-ebpf",
        env!("CARGO_MANIFEST_DIR"),
        detect_ebpf_target()
    )
}

/// Embedded eBPF program bytecode (compiled at build time)
/// This macro ensures proper alignment for eBPF loading
/// The path is determined at compile time based on target architecture
#[cfg(all(not(debug_assertions), target_arch = "x86_64"))]
static EBPF_BYTES: &[u8] = include_bytes_aligned!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/target/bpfel-unknown-none/release/jail-ai-ebpf"
));

#[cfg(all(not(debug_assertions), target_arch = "aarch64"))]
static EBPF_BYTES: &[u8] = include_bytes_aligned!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/target/bpfel-unknown-none/release/jail-ai-ebpf"
));

/// eBPF-based host blocker for containers
///
/// This struct manages eBPF programs that block all packets from containers to host IPs.
/// It attaches eBPF programs to the container's cgroup to filter egress (outgoing) traffic.
///
/// # Requirements
/// - CAP_BPF or root privileges to load eBPF programs
/// - Linux kernel 4.10+ with BPF cgroup_skb support
///
/// # Implementation
/// - **Release builds**: eBPF bytecode is embedded in the binary at compile time
/// - **Debug builds**: eBPF program is loaded from file for easier development
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
    ebpf: Option<Bpf>,
    _links: Vec<CgroupSkbLinkId>,
}

impl EbpfHostBlocker {
    /// Create a new eBPF host blocker instance
    pub fn new() -> Self {
        Self {
            ebpf: None,
            _links: Vec::new(),
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
    /// - **Release mode**: Loads eBPF program from embedded bytecode (compiled at build time)
    /// - **Debug mode**: Loads from file for easier development, falls back to stub mode if not found
    /// - Populates BPF maps with blocked IPs
    /// - Attaches egress filter to cgroup (blocks all outgoing packets to blocked IPs)
    /// - Handles IPv4, IPv6, TCP, UDP, ICMP, and all other packet types
    ///
    /// # Errors
    /// - If BPF program cannot be loaded
    /// - If BPF maps cannot be populated
    /// - If program cannot be attached to cgroup
    /// - If insufficient permissions (requires CAP_BPF or root)
    pub async fn attach_to_cgroup(
        &mut self,
        cgroup_path: &str,
        blocked_ips: &[IpAddr],
    ) -> Result<()> {
        info!(
            "eBPF host blocker: attaching to cgroup {} with {} blocked IPs",
            cgroup_path,
            blocked_ips.len()
        );
        debug!("Blocked IPs: {:?}", blocked_ips);

        // Load eBPF program - use embedded bytes in release, file in debug
        let mut ebpf = {
            #[cfg(not(debug_assertions))]
            {
                info!("Loading embedded eBPF program");
                match Bpf::load(EBPF_BYTES) {
                    Ok(ebpf) => ebpf,
                    Err(e) => {
                        return Err(JailError::Backend(format!(
                            "Failed to load embedded eBPF program: {}",
                            e
                        )))
                    }
                }
            }

            #[cfg(debug_assertions)]
            {
                // In debug mode, try to load from file for easier development
                let ebpf_program_path = get_ebpf_program_path();
                if !std::path::Path::new(&ebpf_program_path).exists() {
                    warn!("⚠️  eBPF program not found at: {}", ebpf_program_path);
                    warn!("   Running in stub mode - host blocking will not be enforced");
                    warn!("   To enable eBPF blocking:");
                    warn!("   1. Install Rust nightly: rustup install nightly");
                    warn!("   2. Install bpf-linker: cargo install bpf-linker");
                    warn!("   3. Build eBPF programs: cargo xtask build-ebpf --release");
                    return Ok(());
                }

                info!("Loading eBPF program from file (debug mode)");
                match Bpf::load_file(&ebpf_program_path) {
                    Ok(ebpf) => ebpf,
                    Err(e) => {
                        return Err(JailError::Backend(format!(
                            "Failed to load eBPF program from file: {}",
                            e
                        )))
                    }
                }
            }
        };

        // Load the eBPF program into the kernel FIRST
        // This creates the maps in the kernel so we can populate them
        debug!("Retrieving and loading block_host_egress program");
        {
            let program: &mut CgroupSkb = ebpf
                .program_mut("block_host_egress")
                .ok_or_else(|| {
                    JailError::Backend(
                        "block_host_egress program not found in eBPF object".to_string(),
                    )
                })?
                .try_into()
                .map_err(|e| {
                    JailError::Backend(format!("Failed to convert to CgroupSkb program: {}", e))
                })?;

            debug!("Loading eBPF program into kernel...");
            program.load().map_err(|e| {
                JailError::Backend(format!(
                    "Failed to load egress program into kernel: {} (errno: {:?})",
                    e,
                    std::io::Error::last_os_error()
                ))
            })?;
            info!("✓ Loaded eBPF program into kernel successfully");
        }

        // Now populate blocked IPv4 addresses (maps exist in kernel now)
        debug!("Retrieving BLOCKED_IPV4 map from eBPF object");
        let map_ref = ebpf.map_mut("BLOCKED_IPV4").ok_or_else(|| {
            JailError::Backend("BLOCKED_IPV4 map not found in eBPF program".to_string())
        })?;

        debug!("Converting to AyaHashMap<u32, u8>");
        let mut blocked_ipv4: AyaHashMap<_, u32, u8> =
            AyaHashMap::try_from(map_ref).map_err(|e| {
                JailError::Backend(format!(
                    "Failed to convert BLOCKED_IPV4 to HashMap: {} (errno: {:?})",
                    e,
                    std::io::Error::last_os_error()
                ))
            })?;

        debug!("Starting IPv4 address insertion");
        let mut ipv4_count = 0;
        for ip in blocked_ips {
            if let IpAddr::V4(ipv4) = ip {
                let ip_u32 = u32::from_be_bytes(ipv4.octets());
                let octets = ipv4.octets();
                debug!(
                    "Inserting IPv4: {} = [{}, {}, {}, {}] = 0x{:08x} (network byte order)",
                    ipv4, octets[0], octets[1], octets[2], octets[3], ip_u32
                );

                blocked_ipv4.insert(ip_u32, 0, 0).map_err(|e| {
                    let os_error = std::io::Error::last_os_error();
                    JailError::Backend(format!(
                        "Failed to insert IPv4 {} (0x{:08x}): {} (errno: {} - {})",
                        ipv4,
                        ip_u32,
                        e,
                        os_error.raw_os_error().unwrap_or(-1),
                        os_error
                    ))
                })?;
                ipv4_count += 1;
                debug!("✓ Successfully inserted IPv4: {}", ipv4);
            }
        }
        info!("✓ Populated {} IPv4 addresses in BPF map", ipv4_count);

        // Populate blocked IPv6 addresses
        debug!("Retrieving BLOCKED_IPV6 map from eBPF object");
        let map_ref_v6 = ebpf.map_mut("BLOCKED_IPV6").ok_or_else(|| {
            JailError::Backend("BLOCKED_IPV6 map not found in eBPF program".to_string())
        })?;

        debug!("Converting to AyaHashMap<[u32; 4], u8>");
        let mut blocked_ipv6: AyaHashMap<_, [u32; 4], u8> = AyaHashMap::try_from(map_ref_v6)
            .map_err(|e| {
                JailError::Backend(format!(
                    "Failed to convert BLOCKED_IPV6 to HashMap: {} (errno: {:?})",
                    e,
                    std::io::Error::last_os_error()
                ))
            })?;

        debug!("Starting IPv6 address insertion");
        let mut ipv6_count = 0;
        for ip in blocked_ips {
            if let IpAddr::V6(ipv6) = ip {
                let ip_u32_array = ipv6_to_u32_array(ipv6);
                debug!("Inserting IPv6: {} ({:08x?})", ipv6, ip_u32_array);

                blocked_ipv6.insert(ip_u32_array, 0, 0).map_err(|e| {
                    let os_error = std::io::Error::last_os_error();
                    JailError::Backend(format!(
                        "Failed to insert IPv6 {}: {} (errno: {} - {})",
                        ipv6,
                        e,
                        os_error.raw_os_error().unwrap_or(-1),
                        os_error
                    ))
                })?;
                ipv6_count += 1;
                debug!("✓ Successfully inserted IPv6: {}", ipv6);
            }
        }
        info!("✓ Populated {} IPv6 addresses in BPF map", ipv6_count);

        // Open cgroup file
        let cgroup_file = File::open(cgroup_path).map_err(|e| {
            JailError::Backend(format!("Failed to open cgroup {}: {}", cgroup_path, e))
        })?;

        let mut links = Vec::new();

        // Attach egress program to cgroup (program already loaded above)
        let program: &mut CgroupSkb = ebpf
            .program_mut("block_host_egress")
            .ok_or_else(|| JailError::Backend("block_host_egress program not found".to_string()))?
            .try_into()
            .map_err(|e| JailError::Backend(format!("Failed to get egress program: {}", e)))?;

        let link = program
            .attach(&cgroup_file, CgroupSkbAttachType::Egress)
            .map_err(|e| JailError::Backend(format!("Failed to attach egress program: {}", e)))?;
        links.push(link);
        info!("Attached egress filtering program to cgroup (blocks IPv4 and IPv6)");

        // Store eBPF instance and links (links will be detached when dropped)
        self.ebpf = Some(ebpf);
        self._links = links;

        info!("✓ eBPF host blocking active for cgroup {}", cgroup_path);
        Ok(())
    }

    /// Detach eBPF program from cgroup
    ///
    /// # Note
    /// This is automatically called when the blocker is dropped.
    /// Links are automatically detached when they go out of scope.
    #[allow(dead_code)]
    pub async fn detach(&mut self) -> Result<()> {
        debug!("eBPF host blocker: detaching programs");

        // Drop links first (this detaches the programs)
        self._links.clear();

        // Drop eBPF instance
        self.ebpf = None;

        info!("eBPF programs detached");
        Ok(())
    }

    /// Check if eBPF program is currently loaded
    #[allow(dead_code)]
    pub fn is_loaded(&self) -> bool {
        self.ebpf.is_some()
    }
}

impl Default for EbpfHostBlocker {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for EbpfHostBlocker {
    fn drop(&mut self) {
        if self.ebpf.is_some() {
            debug!("eBPF host blocker dropped, programs will be automatically detached");
        }
    }
}

/// Convert IPv6 address to array of 4 u32s (network byte order)
fn ipv6_to_u32_array(ipv6: &Ipv6Addr) -> [u32; 4] {
    let octets = ipv6.octets();
    [
        u32::from_be_bytes([octets[0], octets[1], octets[2], octets[3]]),
        u32::from_be_bytes([octets[4], octets[5], octets[6], octets[7]]),
        u32::from_be_bytes([octets[8], octets[9], octets[10], octets[11]]),
        u32::from_be_bytes([octets[12], octets[13], octets[14], octets[15]]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_ebpf_blocker_creation() {
        let blocker = EbpfHostBlocker::new();
        assert!(!blocker.is_loaded());
    }

    #[tokio::test]
    async fn test_ebpf_blocker_attach_fallback() {
        let mut blocker = EbpfHostBlocker::new();
        // Note: 127.0.0.1 is allowed by eBPF program even if in blocked list
        let blocked_ips = vec![
            "10.0.2.2".parse().unwrap(),
            "169.254.169.254".parse().unwrap(),
        ];

        // In debug mode without eBPF compiled: succeeds (stub mode)
        // In debug mode with eBPF compiled or release mode: fails (cgroup doesn't exist)
        // This test just verifies that the fallback logic doesn't panic
        let _result = blocker
            .attach_to_cgroup("/sys/fs/cgroup/test", &blocked_ips)
            .await;
        // Don't assert result - it may fail if cgroup doesn't exist or succeed in stub mode
    }

    #[tokio::test]
    async fn test_ebpf_blocker_detach() {
        let mut blocker = EbpfHostBlocker::new();
        let result = blocker.detach().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_ipv6_to_u32_array() {
        // Test localhost
        let localhost = Ipv6Addr::LOCALHOST; // ::1
        let array = ipv6_to_u32_array(&localhost);
        assert_eq!(array, [0, 0, 0, 1]);

        // Test fe80::1
        let fe80_1: Ipv6Addr = "fe80::1".parse().unwrap();
        let array = ipv6_to_u32_array(&fe80_1);
        assert_eq!(array[0], 0xfe800000);
        assert_eq!(array[1], 0);
        assert_eq!(array[2], 0);
        assert_eq!(array[3], 1);
    }

    #[test]
    fn test_ipv4_to_u32() {
        let ip: Ipv4Addr = "192.168.1.1".parse().unwrap();
        let ip_u32 = u32::from_be_bytes(ip.octets());
        assert_eq!(ip_u32, 0xc0a80101);
    }
}
