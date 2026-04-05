use jail_ai::ebpf::host_ips::{
    get_host_ips, get_ipv4_addresses, get_ipv6_addresses, get_network_interface_ips, parse_hex_ipv6,
};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[test]
fn test_get_host_ips() {
    let ips = get_host_ips().expect("Failed to get host IPs");

    assert!(!ips.is_empty());

    assert!(!ips.contains(&IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
    assert!(!ips.contains(&IpAddr::V6(Ipv6Addr::LOCALHOST)));

    assert!(ips.contains(&IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))));

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
    let hex = "fe800000000000000000000000000001";
    let ip = parse_hex_ipv6(hex).expect("Failed to parse fe80::1");
    assert_eq!(ip, "fe80::1".parse::<Ipv6Addr>().unwrap());
}

#[test]
fn test_parse_hex_ipv6_invalid_length() {
    let hex = "0000000000000001";
    assert!(parse_hex_ipv6(hex).is_err());
}

#[test]
fn test_get_network_interface_ips() {
    let result = get_network_interface_ips();
    match result {
        Ok(ips) => {
            eprintln!("Found {} interface IPs", ips.len());
        }
        Err(e) => {
            eprintln!(
                "Failed to get interface IPs (expected in some environments): {}",
                e
            );
        }
    }
}

#[test]
fn test_ipv4_addresses_parsing() {
    let result = get_ipv4_addresses();
    match result {
        Ok(ips) => {
            eprintln!("Found {} IPv4 addresses", ips.len());
            assert!(!ips.contains(&Ipv4Addr::UNSPECIFIED));
        }
        Err(e) => {
            eprintln!(
                "Failed to get IPv4 addresses (expected in some environments): {}",
                e
            );
        }
    }
}

#[test]
fn test_ipv6_addresses_parsing() {
    let result = get_ipv6_addresses();
    match result {
        Ok(ips) => {
            eprintln!("Found {} IPv6 addresses", ips.len());
        }
        Err(e) => {
            eprintln!(
                "Failed to get IPv6 addresses (expected in some environments): {}",
                e
            );
        }
    }
}
