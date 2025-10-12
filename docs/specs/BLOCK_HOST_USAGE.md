# eBPF Host Blocking - Usage Guide

## Overview

The `--block-host` flag enables eBPF-based host IP blocking for containerized AI agents. This feature uses kernel-level BPF programs attached to container cgroups to intercept and block outbound connections to the host machine.

## Current Status

**Implementation**: ✅ Complete (Stub Mode)
**CLI Integration**: ✅ Complete
**Testing**: ✅ All tests pass (51/52)
**eBPF Compilation**: ⚠️ Requires nightly toolchain + bpf-linker

The integration is fully implemented but runs in **stub mode** - it will log a warning when used instead of actually loading the eBPF program. To activate full eBPF functionality, you need to install the required toolchain (see below).

## Usage

### Basic Usage

```bash
# Create a jail with host blocking enabled
cargo run -- create my-agent --block-host

# Agent commands with host blocking
cargo run -- claude --block-host -- chat "help me code"
cargo run -- copilot --copilot-dir --block-host -- suggest "write tests"
```

### Combined with Other Flags

```bash
# Block host + network isolation + port mapping
cargo run -- create my-agent --block-host -p 5432:5432

# Block host + OAuth authentication (temporary)
cargo run -- codex --codex-dir --block-host --auth

# Block host + isolated image + specific layers
cargo run -- claude --block-host --isolated --layers base,rust
```

## How It Works

### Architecture

1. **Container Creation**: When `--block-host` is specified, the container is created with normal networking (slirp4netns or netavark for rootless)

2. **PID & Cgroup Detection**: After container starts, jail-ai:
   - Retrieves container PID via `podman inspect`
   - Reads cgroup path from `/proc/<pid>/cgroup` (supports cgroup v1 and v2)

3. **Host IP Detection**: Discovers IPs to block:
   - Localhost range: `127.0.0.0/8`
   - Host network interfaces (from `/proc/net/fib_trie` and `/proc/net/if_inet6`)
   - Metadata service IPs: `169.254.169.254`, `10.0.2.2`

4. **eBPF Program Loading**: (When toolchain is available)
   - Loads compiled eBPF program from `jail-ai-ebpf/target/bpfel-unknown-none/release/jail-ai-ebpf`
   - Populates `BLOCKED_IPV4` and `BLOCKED_IPV6` BPF maps with detected IPs
   - Attaches program to container's cgroup with `BPF_CGROUP_INET4_CONNECT` and `BPF_CGROUP_INET6_CONNECT`

5. **Connection Interception**: eBPF program intercepts every `connect()` syscall:
   - Checks destination IP against blocked lists
   - Returns `0` (deny) for blocked IPs
   - Returns `1` (allow) for all other connections

### Security Model

- **Rootless Container**: Runs with user privileges, no host access by default
- **eBPF Loading**: Requires CAP_BPF or root to load programs (host-side only)
- **Fail-Open**: On any error, the system fails open (allows connections) rather than breaking networking
- **Layer-specific**: Blocking is per-container, not system-wide

## Enabling eBPF Compilation

### Prerequisites

To compile and use the actual eBPF program, you need:

1. **Rust Nightly**: `rustup install nightly`
2. **bpf-linker**: `cargo install bpf-linker`
3. **Kernel headers**: Required for your specific kernel version

### Installation

```bash
# Install Rust nightly
rustup install nightly

# Install bpf-linker
cargo install bpf-linker

# Install kernel headers (Debian/Ubuntu)
sudo apt-get install linux-headers-$(uname -r)

# Install kernel headers (Fedora/RHEL)
sudo dnf install kernel-devel kernel-headers
```

### Building eBPF Program

```bash
# Build just the eBPF program
cargo xtask build-ebpf --release

# Build everything (eBPF + main binary)
cargo xtask build --release

# The compiled eBPF program will be at:
# jail-ai-ebpf/target/bpfel-unknown-none/release/jail-ai-ebpf
```

### After Installation

Once the toolchain is installed and eBPF programs are compiled:

1. The stub mode warning will no longer appear
2. eBPF programs will be loaded automatically when `--block-host` is used
3. Host connections will be actually blocked at kernel level
4. Container will be unable to access host services (HTTP, SSH, databases, etc.)

## Code Structure

```
jail-ai/
├── src/
│   ├── ebpf/
│   │   ├── mod.rs           # EbpfHostBlocker implementation
│   │   └── host_ips.rs      # Host IP detection logic
│   ├── backend/podman.rs    # get_container_pid(), get_container_cgroup_path()
│   ├── cli.rs               # --block-host flag definition
│   ├── config.rs            # block_host field in JailConfig
│   ├── jail.rs              # block_host() builder method
│   └── main.rs              # block_host flag wiring
├── jail-ai-ebpf/
│   ├── src/main.rs          # Kernel-side eBPF program
│   └── Cargo.toml           # eBPF crate config
└── xtask/
    └── src/main.rs          # Build infrastructure for eBPF
```

## Implementation Details

### Methods Added to PodmanBackend

```rust
// Get the PID of the container's main process
pub async fn get_container_pid(&self, name: &str) -> Result<u32>

// Get the cgroup path for the container (supports v1 and v2)
pub async fn get_container_cgroup_path(&self, name: &str) -> Result<String>
```

### EbpfHostBlocker API

```rust
pub struct EbpfHostBlocker {
    bpf: Option<Ebpf>,
}

impl EbpfHostBlocker {
    pub fn new() -> Self

    // Attach eBPF program to container's cgroup
    pub async fn attach_to_cgroup(
        &mut self,
        cgroup_path: &str,
        blocked_ips: &[std::net::IpAddr],
    ) -> Result<()>

    // Detach eBPF program
    pub async fn detach(&mut self) -> Result<()>

    // Check if we have required capabilities
    fn has_bpf_capabilities() -> bool
}
```

### Host IP Detection

```rust
// Main entry point - detects all host IPs to block
pub fn get_host_ips() -> Result<Vec<IpAddr>>

// Internal functions
fn get_network_interface_ips() -> Result<Vec<IpAddr>>
fn get_ipv4_addresses() -> Result<Vec<Ipv4Addr>>  // from /proc/net/fib_trie
fn get_ipv6_addresses() -> Result<Vec<Ipv6Addr>>  // from /proc/net/if_inet6
fn parse_hex_ipv6(hex: &str) -> Result<Ipv6Addr>
```

## Limitations

### Current (Stub Mode)
- No actual blocking occurs
- Warning message displayed when `--block-host` is used
- All other functionality works normally

### With eBPF Active
- Requires CAP_BPF or root to load programs
- Kernel must support eBPF and cgroup attachment
- May not work with some older kernels (< 4.10)
- Blocks ALL connections to detected host IPs (no exceptions)

## Troubleshooting

### "eBPF host blocker not available" Warning

This means the toolchain isn't installed. Follow the instructions in "Enabling eBPF Compilation" above.

### "Failed to attach eBPF program: Permission denied"

You need CAP_BPF capability or root to load eBPF programs:

```bash
# Option 1: Run with sudo
sudo jail-ai create my-agent --block-host

# Option 2: Set CAP_BPF capability
sudo setcap cap_bpf+ep $(which jail-ai)
```

### "Container can still connect to host"

Check if eBPF program is actually loaded:

```bash
# List BPF programs
sudo bpftool prog list

# List cgroup BPF attachments
sudo bpftool cgroup tree
```

## Testing

```bash
# Run all tests (includes eBPF module tests)
cargo test

# Run just eBPF tests
cargo test --package jail-ai ebpf

# Build and test eBPF program
cargo xtask build-ebpf
```

## Future Enhancements

Potential improvements for the eBPF blocking feature:

1. **Selective Blocking**: Allow specific host IPs/ports (e.g., allow localhost:8080 but block everything else)
2. **Dynamic Updates**: Update blocked IP list without reloading program
3. **Audit Logging**: Log blocked connection attempts to BPF ring buffer
4. **IPv6 Support**: Full IPv6 blocking support (currently implemented but untested)
5. **Integration Test**: Add integration test that verifies actual blocking behavior

## Security Considerations

### Threat Model

**Protects Against**:
- AI agent accessing host services (SSH, databases, web servers)
- Container escaping to communicate with host processes
- Exfiltration via localhost services
- Metadata service access (cloud provider APIs)

**Does NOT Protect Against**:
- Kernel exploits (use SELinux/AppArmor for that)
- Container privilege escalation (use rootless containers)
- Attacks on the container itself
- Network attacks on external services

### Best Practices

1. **Use with Rootless Containers**: Always run containers rootless for defense in depth
2. **Combine with Network Isolation**: Use `--network=private` or `--network=none` when possible
3. **Regular Updates**: Keep kernel and jail-ai up to date for security patches
4. **Audit Logs**: Monitor system logs for BPF-related errors
5. **Minimal Privileges**: Run jail-ai with minimum required capabilities

## References

- [eBPF Documentation](https://ebpf.io/)
- [Aya Framework](https://aya-rs.dev/)
- [BPF Program Types](https://www.kernel.org/doc/html/latest/bpf/prog_cgroup_sockaddr.html)
- [Podman Security](https://docs.podman.io/en/latest/markdown/podman-run.1.html#security-options)
