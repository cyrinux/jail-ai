//! IPC protocol for communication between jail-ai and jail-ai-ebpf-loader

use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// Request from jail-ai to load eBPF program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadRequest {
    /// Path to the container's cgroup
    pub cgroup_path: String,
    /// List of IP addresses to block
    pub blocked_ips: Vec<IpAddr>,
}

/// Response from loader to jail-ai
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadResponse {
    pub success: bool,
    pub message: String,
    /// Link IDs for the attached programs (for cleanup)
    pub link_ids: Vec<u64>,
}
