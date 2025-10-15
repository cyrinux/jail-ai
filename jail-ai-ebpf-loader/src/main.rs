//! jail-ai-ebpf-loader - Privileged helper for loading eBPF programs
//!
//! This is a minimal setuid helper binary that:
//! 1. Loads eBPF programs into the kernel
//! 2. Populates BPF maps with blocked IPs
//! 3. Attaches programs to cgroups
//! 4. Drops privileges immediately after loading
//! 5. Returns control to the unprivileged jail-ai process
//!
//! Security considerations:
//! - Validates all inputs rigorously
//! - Minimal attack surface (< 500 LOC)
//! - Drops capabilities after loading
//! - No network access, no file writes beyond BPF operations
//! - Stateless: exits immediately after loading

use aya::{
    include_bytes_aligned,
    maps::HashMap as AyaHashMap,
    programs::{CgroupSkb, CgroupSkbAttachType},
    Bpf,
};
use std::fs::File;
use std::io::{self, Read};
use std::net::{IpAddr, Ipv6Addr};
use tracing::{debug, error, info, warn};

mod protocol;
use protocol::{LoadRequest, LoadResponse};

/// Embedded eBPF program bytecode (compiled at build time)
/// Note: The eBPF program must be built BEFORE building this helper binary
/// Path is relative to workspace root
#[cfg(all(not(debug_assertions), target_arch = "x86_64"))]
static EBPF_BYTES: &[u8] = include_bytes_aligned!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../target/bpfel-unknown-none/release/jail-ai-ebpf"
));

#[cfg(all(not(debug_assertions), target_arch = "aarch64"))]
static EBPF_BYTES: &[u8] = include_bytes_aligned!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../target/bpfel-unknown-none/release/jail-ai-ebpf"
));

/// Path to eBPF program in debug mode
#[cfg(debug_assertions)]
fn get_ebpf_program_path() -> String {
    format!(
        "{}/target/bpfel-unknown-none/release/jail-ai-ebpf",
        env!("CARGO_MANIFEST_DIR")
    )
}

#[tokio::main]
async fn main() {
    // Initialize logging to stderr only (stdout is used for JSON response)
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("jail-ai-ebpf-loader starting");

    // Verify we have the required capabilities
    if let Err(e) = verify_capabilities() {
        error!("Missing required capabilities: {}", e);
        std::process::exit(1);
    }

    // Read request from stdin (JSON)
    let request = match read_request() {
        Ok(req) => req,
        Err(e) => {
            error!("Failed to read request: {}", e);
            send_response(LoadResponse {
                success: false,
                message: format!("Failed to read request: {}", e),
                link_ids: vec![],
            });
            std::process::exit(1);
        }
    };

    info!("Received request to load eBPF for cgroup: {}", request.cgroup_path);
    debug!("Blocking {} IPs", request.blocked_ips.len());

    // Validate inputs
    if let Err(e) = validate_request(&request) {
        error!("Invalid request: {}", e);
        send_response(LoadResponse {
            success: false,
            message: format!("Invalid request: {}", e),
            link_ids: vec![],
        });
        std::process::exit(1);
    }

    // Load and attach eBPF program
    match load_and_attach_ebpf(request).await {
        Ok(link_ids) => {
            info!("Successfully loaded and attached eBPF programs");
            send_response(LoadResponse {
                success: true,
                message: "eBPF programs loaded successfully".to_string(),
                link_ids,
            });

            // Drop capabilities before exiting
            if let Err(e) = drop_capabilities() {
                warn!("Failed to drop capabilities: {}", e);
            }

            std::process::exit(0);
        }
        Err(e) => {
            error!("Failed to load eBPF: {}", e);
            send_response(LoadResponse {
                success: false,
                message: format!("Failed to load eBPF: {}", e),
                link_ids: vec![],
            });
            std::process::exit(1);
        }
    }
}

/// Read LoadRequest from stdin as JSON
fn read_request() -> io::Result<LoadRequest> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;

    let request: LoadRequest = serde_json::from_str(&buffer)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    Ok(request)
}

/// Send LoadResponse to stdout as JSON
fn send_response(response: LoadResponse) {
    if let Ok(json) = serde_json::to_string(&response) {
        println!("{}", json);
    } else {
        error!("Failed to serialize response");
    }
}

/// Verify that we have CAP_BPF and CAP_NET_ADMIN
fn verify_capabilities() -> Result<(), String> {
    // Check if we're root or have the required capabilities
    let euid = unsafe { libc::geteuid() };
    if euid == 0 {
        debug!("Running as root, capabilities available");
        return Ok(());
    }

    // Check for capabilities using caps crate
    match caps::has_cap(None, caps::CapSet::Effective, caps::Capability::CAP_BPF) {
        Ok(true) => debug!("CAP_BPF available"),
        Ok(false) => return Err("CAP_BPF not available".to_string()),
        Err(e) => return Err(format!("Failed to check CAP_BPF: {}", e)),
    }

    match caps::has_cap(None, caps::CapSet::Effective, caps::Capability::CAP_NET_ADMIN) {
        Ok(true) => debug!("CAP_NET_ADMIN available"),
        Ok(false) => return Err("CAP_NET_ADMIN not available".to_string()),
        Err(e) => return Err(format!("Failed to check CAP_NET_ADMIN: {}", e)),
    }

    Ok(())
}

/// Validate the request to prevent malicious inputs
fn validate_request(request: &LoadRequest) -> Result<(), String> {
    // Validate cgroup path
    if request.cgroup_path.is_empty() {
        return Err("cgroup_path cannot be empty".to_string());
    }

    if !request.cgroup_path.starts_with("/sys/fs/cgroup") {
        return Err("cgroup_path must start with /sys/fs/cgroup".to_string());
    }

    // Check path doesn't contain suspicious sequences
    if request.cgroup_path.contains("..") || request.cgroup_path.contains("//") {
        return Err("cgroup_path contains invalid sequences".to_string());
    }

    // Validate cgroup path exists
    if !std::path::Path::new(&request.cgroup_path).exists() {
        return Err(format!("cgroup path does not exist: {}", request.cgroup_path));
    }

    // Validate IP addresses (basic sanity check)
    if request.blocked_ips.is_empty() {
        return Err("blocked_ips cannot be empty".to_string());
    }

    if request.blocked_ips.len() > 1000 {
        return Err("blocked_ips exceeds maximum (1000)".to_string());
    }

    Ok(())
}

/// Load eBPF program and attach to cgroup
async fn load_and_attach_ebpf(request: LoadRequest) -> Result<Vec<u64>, String> {
    // Load eBPF program
    let mut ebpf = load_ebpf_program()?;

    // Load the program into the kernel first
    {
        let program: &mut CgroupSkb = ebpf
            .program_mut("block_host_egress")
            .ok_or_else(|| "block_host_egress program not found in eBPF object".to_string())?
            .try_into()
            .map_err(|e| format!("Failed to convert to CgroupSkb program: {}", e))?;

        program
            .load()
            .map_err(|e| format!("Failed to load egress program into kernel: {}", e))?;

        info!("✓ Loaded eBPF program into kernel");
    }

    // Populate blocked IPv4 addresses
    populate_blocked_ipv4(&mut ebpf, &request.blocked_ips)?;

    // Populate blocked IPv6 addresses
    populate_blocked_ipv6(&mut ebpf, &request.blocked_ips)?;

    // Open cgroup file
    let cgroup_file = File::open(&request.cgroup_path)
        .map_err(|e| format!("Failed to open cgroup {}: {}", request.cgroup_path, e))?;

    // Attach egress program to cgroup
    let program: &mut CgroupSkb = ebpf
        .program_mut("block_host_egress")
        .ok_or_else(|| "block_host_egress program not found".to_string())?
        .try_into()
        .map_err(|e| format!("Failed to get egress program: {}", e))?;

    let link = program
        .attach(&cgroup_file, CgroupSkbAttachType::Egress)
        .map_err(|e| format!("Failed to attach egress program: {}", e))?;

    info!("✓ Attached egress filtering program to cgroup");

    // Keep the link and ebpf alive by leaking them
    // The eBPF program will remain active until the container/cgroup is destroyed
    std::mem::forget(link);
    std::mem::forget(ebpf);

    // Return empty link IDs since we can't access them after forgetting
    Ok(vec![])
}

/// Load eBPF program from embedded bytes or file
fn load_ebpf_program() -> Result<Bpf, String> {
    #[cfg(not(debug_assertions))]
    {
        info!("Loading embedded eBPF program");
        Bpf::load(EBPF_BYTES).map_err(|e| format!("Failed to load embedded eBPF program: {}", e))
    }

    #[cfg(debug_assertions)]
    {
        let ebpf_program_path = get_ebpf_program_path();
        if !std::path::Path::new(&ebpf_program_path).exists() {
            return Err(format!("eBPF program not found at: {}", ebpf_program_path));
        }

        info!("Loading eBPF program from file (debug mode)");
        Bpf::load_file(&ebpf_program_path)
            .map_err(|e| format!("Failed to load eBPF program from file: {}", e))
    }
}

/// Populate BLOCKED_IPV4 map
fn populate_blocked_ipv4(ebpf: &mut Bpf, blocked_ips: &[IpAddr]) -> Result<(), String> {
    let map_ref = ebpf
        .map_mut("BLOCKED_IPV4")
        .ok_or_else(|| "BLOCKED_IPV4 map not found in eBPF program".to_string())?;

    let mut blocked_ipv4: AyaHashMap<_, u32, u8> = AyaHashMap::try_from(map_ref)
        .map_err(|e| format!("Failed to convert BLOCKED_IPV4 to HashMap: {}", e))?;

    let mut ipv4_count = 0;
    for ip in blocked_ips {
        if let IpAddr::V4(ipv4) = ip {
            let ip_u32 = u32::from_be_bytes(ipv4.octets());
            blocked_ipv4
                .insert(ip_u32, 0, 0)
                .map_err(|e| format!("Failed to insert IPv4 {}: {}", ipv4, e))?;
            ipv4_count += 1;
        }
    }
    info!("✓ Populated {} IPv4 addresses in BPF map", ipv4_count);

    Ok(())
}

/// Populate BLOCKED_IPV6 map
fn populate_blocked_ipv6(ebpf: &mut Bpf, blocked_ips: &[IpAddr]) -> Result<(), String> {
    let map_ref_v6 = ebpf
        .map_mut("BLOCKED_IPV6")
        .ok_or_else(|| "BLOCKED_IPV6 map not found in eBPF program".to_string())?;

    let mut blocked_ipv6: AyaHashMap<_, [u32; 4], u8> = AyaHashMap::try_from(map_ref_v6)
        .map_err(|e| format!("Failed to convert BLOCKED_IPV6 to HashMap: {}", e))?;

    let mut ipv6_count = 0;
    for ip in blocked_ips {
        if let IpAddr::V6(ipv6) = ip {
            let ip_u32_array = ipv6_to_u32_array(ipv6);
            blocked_ipv6
                .insert(ip_u32_array, 0, 0)
                .map_err(|e| format!("Failed to insert IPv6 {}: {}", ipv6, e))?;
            ipv6_count += 1;
        }
    }
    info!("✓ Populated {} IPv6 addresses in BPF map", ipv6_count);

    Ok(())
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

/// Drop all capabilities after loading eBPF
fn drop_capabilities() -> Result<(), String> {
    info!("Dropping capabilities");

    // Drop all capabilities from all sets
    caps::clear(None, caps::CapSet::Effective)
        .map_err(|e| format!("Failed to clear effective caps: {}", e))?;
    caps::clear(None, caps::CapSet::Permitted)
        .map_err(|e| format!("Failed to clear permitted caps: {}", e))?;
    caps::clear(None, caps::CapSet::Inheritable)
        .map_err(|e| format!("Failed to clear inheritable caps: {}", e))?;

    info!("✓ All capabilities dropped");
    Ok(())
}
