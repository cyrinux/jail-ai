use crate::error::{JailError, Result};
use std::collections::HashSet;
use std::fs;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use tracing::{debug, warn};

/// Get all host IP addresses that should be blocked from container access
///
/// This function detects:
/// - Host network interfaces (from /proc/net/fib_trie and /proc/net/if_inet6)
/// - Metadata service IPs (169.254.169.254, 10.0.2.2 for VMs)
/// - Container host gateway IPs (169.254.1.1, 169.254.1.2 for podman host.containers.internal)
///
/// Note: Localhost (127.0.0.0/8 and ::1) is NOT blocked - it's explicitly allowed in the eBPF program
///
/// # Returns
/// Vec of IP addresses to block
///
/// # Errors
/// Returns Err if unable to read network information from /proc
pub fn get_host_ips() -> Result<Vec<IpAddr>> {
    let mut ips = HashSet::new();

    // Note: Localhost (127.0.0.0/8 and ::1) is allowed by eBPF program, not added to blocked list

    // Add metadata service IPs (cloud providers)
    ips.insert(IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))); // AWS/Azure metadata
    ips.insert(IpAddr::V4(Ipv4Addr::new(10, 0, 2, 2))); // QEMU/VirtualBox gateway

    // Add container host gateway IPs (podman/containers host access)
    // host.containers.internal typically resolves to one of these
    ips.insert(IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1))); // Podman host gateway
    ips.insert(IpAddr::V4(Ipv4Addr::new(169, 254, 1, 2))); // Podman host.containers.internal
    ips.insert(IpAddr::V4(Ipv4Addr::new(192, 168, 65, 2))); // Docker Desktop for Mac

    // Get network interface IPs
    match get_network_interface_ips() {
        Ok(interface_ips) => {
            debug!("Found {} network interface IPs", interface_ips.len());
            for ip in interface_ips {
                ips.insert(ip);
            }
        }
        Err(e) => {
            warn!("Failed to get network interface IPs: {}", e);
            // Continue with at least localhost blocking
        }
    }

    let ip_list: Vec<IpAddr> = ips.into_iter().collect();
    debug!("Total host IPs to block: {}", ip_list.len());
    for ip in &ip_list {
        debug!("  - {}", ip);
    }

    Ok(ip_list)
}

/// Get IP addresses from network interfaces by reading /proc
fn get_network_interface_ips() -> Result<Vec<IpAddr>> {
    let mut ips = Vec::new();

    // Get IPv4 addresses from /proc/net/fib_trie
    match get_ipv4_addresses() {
        Ok(ipv4_addrs) => {
            ips.extend(ipv4_addrs.into_iter().map(IpAddr::V4));
        }
        Err(e) => {
            warn!("Failed to read IPv4 addresses: {}", e);
        }
    }

    // Get IPv6 addresses from /proc/net/if_inet6
    match get_ipv6_addresses() {
        Ok(ipv6_addrs) => {
            ips.extend(ipv6_addrs.into_iter().map(IpAddr::V6));
        }
        Err(e) => {
            warn!("Failed to read IPv6 addresses: {}", e);
        }
    }

    Ok(ips)
}

/// Parse IPv4 addresses from /proc/net/fib_trie
///
/// This file contains routing table information in a tree format.
/// We extract IP addresses that appear in the "Local" entries.
fn get_ipv4_addresses() -> Result<Vec<Ipv4Addr>> {
    let content = fs::read_to_string("/proc/net/fib_trie")
        .map_err(|e| JailError::Backend(format!("Failed to read /proc/net/fib_trie: {}", e)))?;

    let mut ips = HashSet::new();
    let mut prev_line = String::new();

    for line in content.lines() {
        // Look for lines that contain IP addresses in Local entries
        // Format: "      /32 host LOCAL"
        // The line before contains the IP like "    |-- 192.168.1.1"
        if line.contains("/32 host LOCAL") || line.contains("host LOCAL") {
            // Try to find the IP in the previous line or same line
            // Parse format like "|-- 192.168.1.1"
            if let Some(ip_part) = prev_line.split_whitespace().find(|s| s.contains('.')) {
                if let Ok(ip) = ip_part.parse::<Ipv4Addr>() {
                    if should_block_ipv4(&ip) {
                        ips.insert(ip);
                    }
                }
            }
            if let Some(ip_part) = line.split_whitespace().find(|s| s.contains('.')) {
                if let Ok(ip) = ip_part.parse::<Ipv4Addr>() {
                    if should_block_ipv4(&ip) {
                        ips.insert(ip);
                    }
                }
            }
        }
        prev_line = line.to_string();
    }

    Ok(ips.into_iter().collect())
}

/// Determine if an IPv4 address should be blocked
///
/// We filter out some common ranges that are safe to allow or not useful to block:
/// - Multicast (224.0.0.0/4)
/// - Broadcast (255.255.255.255)
/// - Link-local container IPs that are not the host gateway
fn should_block_ipv4(ip: &Ipv4Addr) -> bool {
    // Skip unspecified, multicast, or broadcast
    if ip.is_unspecified() || ip.is_multicast() || ip.is_broadcast() {
        return false;
    }

    // Skip localhost (127.0.0.0/8) - it's explicitly allowed by eBPF program
    if ip.is_loopback() {
        return false;
    }

    let octets = ip.octets();

    // Container bridge IPs (172.16.0.0/12 range used by Docker/Podman bridges)
    // Block only if it looks like a bridge gateway (typically .0.1 or .0.0.1)
    if octets[0] == 172 && octets[1] >= 16 && octets[1] < 32 {
        return octets[2] == 0 && (octets[3] == 1 || octets[3] == 0);
    }

    // Podman/docker internal container IPs (10.88.0.0/16, 10.89.0.0/16, etc.)
    // Block only if it looks like a gateway (typically .0.1)
    if octets[0] == 10 && octets[1] >= 88 && octets[1] <= 91 {
        return octets[2] == 0 && octets[3] == 1;
    }

    // Block everything else
    true
}

/// Parse IPv6 addresses from /proc/net/if_inet6
///
/// Format: <ipv6_addr> <iface_idx> <prefix_len> <scope> <flags> <iface_name>
/// Example: 00000000000000000000000000000001 01 80 10 80 lo
///          (represents ::1/128)
fn get_ipv6_addresses() -> Result<Vec<Ipv6Addr>> {
    let content = fs::read_to_string("/proc/net/if_inet6")
        .map_err(|e| JailError::Backend(format!("Failed to read /proc/net/if_inet6: {}", e)))?;

    let mut ips = HashSet::new();

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        // First part is the hex-encoded IPv6 address (32 hex digits)
        let hex_addr = parts[0];
        if hex_addr.len() != 32 {
            continue;
        }

        match parse_hex_ipv6(hex_addr) {
            Ok(ip) => {
                if should_block_ipv6(&ip) {
                    ips.insert(ip);
                }
            }
            Err(e) => {
                warn!("Failed to parse IPv6 address '{}': {}", hex_addr, e);
            }
        }
    }

    Ok(ips.into_iter().collect())
}

/// Determine if an IPv6 address should be blocked
///
/// We filter out some common ranges that are safe to allow or not useful to block:
/// - Multicast (ff00::/8)
/// - Link-local container IPs
fn should_block_ipv6(ip: &Ipv6Addr) -> bool {
    // Skip unspecified
    if ip.is_unspecified() {
        return false;
    }

    // Skip localhost (::1) - it's explicitly allowed by eBPF program
    if ip.is_loopback() {
        return false;
    }

    // Skip multicast (ff00::/8)
    if ip.is_multicast() {
        return false;
    }

    // Allow ULA (Unique Local Address) container ranges (fd00::/8)
    // These are typically used for container networking
    let segments = ip.segments();
    if segments[0] & 0xff00 == 0xfd00 {
        return false;
    }

    // Block everything else (including link-local fe80::/10 which could be host)
    true
}

/// Parse hex-encoded IPv6 address from /proc/net/if_inet6
///
/// Input format: 32 hex digits without separators (e.g., "00000000000000000000000000000001")
/// Output: Ipv6Addr (e.g., ::1)
fn parse_hex_ipv6(hex: &str) -> Result<Ipv6Addr> {
    if hex.len() != 32 {
        return Err(JailError::Backend(format!(
            "Invalid hex IPv6 length: {} (expected 32)",
            hex.len()
        )));
    }

    // Parse as 8 groups of 4 hex digits (16-bit segments)
    let mut segments = [0u16; 8];
    for (i, segment) in segments.iter_mut().enumerate() {
        let start = i * 4;
        let end = start + 4;
        let hex_segment = &hex[start..end];

        *segment = u16::from_str_radix(hex_segment, 16).map_err(|e| {
            JailError::Backend(format!(
                "Failed to parse hex segment '{}': {}",
                hex_segment, e
            ))
        })?;
    }

    Ok(Ipv6Addr::new(
        segments[0],
        segments[1],
        segments[2],
        segments[3],
        segments[4],
        segments[5],
        segments[6],
        segments[7],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_host_ips() {
        let ips = get_host_ips().expect("Failed to get host IPs");

        // Should have metadata and gateway IPs
        assert!(!ips.is_empty());

        // Should NOT include localhost (it's allowed by eBPF)
        assert!(!ips.contains(&IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
        assert!(!ips.contains(&IpAddr::V6(Ipv6Addr::LOCALHOST)));

        // Should include metadata service IPs
        assert!(ips.contains(&IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))));

        // Should include container host gateway IPs
        assert!(ips.contains(&IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1))));
        assert!(ips.contains(&IpAddr::V4(Ipv4Addr::new(169, 254, 1, 2))));
    }

    #[test]
    fn test_parse_hex_ipv6_localhost() {
        let hex = "00000000000000000000000000000001";
        let ip = parse_hex_ipv6(hex).expect("Failed to parse localhost");
        assert_eq!(ip, Ipv6Addr::LOCALHOST);
    }

    #[test]
    fn test_parse_hex_ipv6_example() {
        // fe80::1 = fe80:0000:0000:0000:0000:0000:0000:0001
        let hex = "fe800000000000000000000000000001";
        let ip = parse_hex_ipv6(hex).expect("Failed to parse fe80::1");
        assert_eq!(ip, "fe80::1".parse::<Ipv6Addr>().unwrap());
    }

    #[test]
    fn test_parse_hex_ipv6_invalid_length() {
        let hex = "0000000000000001"; // Too short
        assert!(parse_hex_ipv6(hex).is_err());
    }

    #[test]
    fn test_get_network_interface_ips() {
        // This test depends on the system having network interfaces
        // It should at least not crash
        let result = get_network_interface_ips();
        // May succeed or fail depending on /proc availability
        // Just ensure it doesn't panic
        match result {
            Ok(ips) => {
                debug!("Found {} interface IPs", ips.len());
            }
            Err(e) => {
                debug!(
                    "Failed to get interface IPs (expected in some environments): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_ipv4_addresses_parsing() {
        // This test depends on system state
        // Just ensure it doesn't panic
        let result = get_ipv4_addresses();
        match result {
            Ok(ips) => {
                debug!("Found {} IPv4 addresses", ips.len());
                // Should not contain unspecified
                assert!(!ips.contains(&Ipv4Addr::UNSPECIFIED));
            }
            Err(e) => {
                debug!(
                    "Failed to get IPv4 addresses (expected in some environments): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_ipv6_addresses_parsing() {
        // This test depends on system state
        // Just ensure it doesn't panic
        let result = get_ipv6_addresses();
        match result {
            Ok(ips) => {
                debug!("Found {} IPv6 addresses", ips.len());
            }
            Err(e) => {
                debug!(
                    "Failed to get IPv6 addresses (expected in some environments): {}",
                    e
                );
            }
        }
    }
}
