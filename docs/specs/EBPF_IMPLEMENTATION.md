# eBPF Host Blocking Implementation

## Overview

This document describes the implementation of eBPF-based host blocking for jail-ai containers. The goal is to use cgroup-attached eBPF programs to intercept and block `connect()` syscalls from containers to host IP addresses, providing an additional layer of network isolation.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Host System                              │
│                                                              │
│  ┌──────────────┐                                           │
│  │  jail-ai     │                                           │
│  │  (Rust)      │                                           │
│  └──────┬───────┘                                           │
│         │                                                    │
│         │ 1. Creates container                              │
│         │ 2. Gets PID and cgroup path                       │
│         │ 3. Loads eBPF program                             │
│         │ 4. Attaches to cgroup                             │
│         ▼                                                    │
│  ┌──────────────────────────────────┐                       │
│  │  eBPF Program (loaded in kernel) │                       │
│  │  - BPF_PROG_TYPE_CGROUP_SOCK_ADDR│                       │
│  │  - Hook: BPF_CGROUP_INET4_CONNECT│                       │
│  │  - Hook: BPF_CGROUP_INET6_CONNECT│                       │
│  └──────────────┬───────────────────┘                       │
│                 │                                            │
│                 │ Intercepts connect() syscalls              │
│                 ▼                                            │
│  ┌─────────────────────────────────────────────┐            │
│  │        Container (in cgroup)                │            │
│  │  ┌──────────────┐                           │            │
│  │  │ AI Agent     │  ──X──> connect() to host │            │
│  │  │ (Claude etc) │  ──✓──> connect() to web  │            │
│  │  └──────────────┘                           │            │
│  └─────────────────────────────────────────────┘            │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Status

### ✅ Completed

1. **Container PID and cgroup path retrieval** (src/backend/podman.rs:63-129)
   - `get_container_pid()` - Retrieves container PID via podman inspect
   - `get_container_cgroup_path()` - Reads `/proc/<pid>/cgroup` to get cgroup path
   - Supports both cgroup v1 and v2

2. **eBPF module structure** (src/ebpf/mod.rs)
   - `EbpfHostBlocker` struct for managing eBPF programs
   - Added Aya dependencies to Cargo.toml
   - Basic API design for attach/detach operations

### 🔄 In Progress / Remaining Work

The following steps are needed to complete the eBPF host-blocking feature:

#### 1. Create eBPF Program Crate

Create a separate crate for the eBPF program itself:

```
jail-ai-ebpf/
├── Cargo.toml
└── src/
    └── main.rs  # eBPF program in Rust using aya-bpf
```

The eBPF program needs to:
- Hook `connect()` syscalls using `BPF_CGROUP_INET4_CONNECT` and `BPF_CGROUP_INET6_CONNECT`
- Check destination IP against a BPF map of blocked IPs
- Return 0 (allow) or -EPERM (deny) based on the check

Example structure:
```rust
#[cgroup_sock_addr(connect4)]
pub fn block_host_connect(ctx: SockAddrContext) -> i32 {
    // Get destination IP from context
    // Check against blocked_ips BPF map
    // Return 0 (allow) or -EPERM (deny)
}
```

#### 2. Set Up Build Infrastructure

Options:
- **xtask pattern**: Create `xtask/` directory with custom build logic
- **build.rs**: Add build script to compile eBPF program
- **Makefile**: Add make targets for eBPF compilation

Requirements:
- Install `bpf-linker`: `cargo install bpf-linker`
- Install `rustup target add bpfel-unknown-none` (for eBPF target)

#### 3. Implement Host IP Detection

Create a function to detect host IP addresses that should be blocked:

```rust
pub fn get_host_ips() -> Result<Vec<IpAddr>> {
    // Read network interfaces
    // Identify host-accessible IPs:
    // - localhost (127.0.0.1, ::1)
    // - Host's private network IPs
    // - Gateway IPs (if applicable)
}
```

#### 4. Complete eBPF Loader Implementation

Enhance `src/ebpf/mod.rs` to:
- Load compiled eBPF bytecode (embedded in binary or from file)
- Populate BPF maps with blocked IPs
- Attach program to cgroup using Aya's `CgroupSockAddr` program type
- Handle cleanup and detachment

Example:
```rust
pub async fn attach_to_cgroup(&mut self, cgroup_path: &str, blocked_ips: &[IpAddr]) -> Result<()> {
    // 1. Load eBPF program from embedded bytecode
    let mut bpf = Ebpf::load(include_bytes_aligned!("../target/bpfel-unknown-none/release/jail-ai-ebpf"))?;

    // 2. Populate blocked_ips BPF map
    let blocked_ips_map: HashMap<_, u32, IpAddr> = HashMap::try_from(bpf.map_mut("blocked_ips")?)?;
    for (idx, ip) in blocked_ips.iter().enumerate() {
        blocked_ips_map.insert(idx as u32, ip, 0)?;
    }

    // 3. Attach to cgroup
    let program: &mut CgroupSockAddr = bpf.program_mut("block_host_connect")?.try_into()?;
    program.load()?;
    program.attach(cgroup_path)?;

    self.bpf = Some(bpf);
    Ok(())
}
```

#### 5. Integrate into Jail Creation Workflow

Modify `src/backend/podman.rs` or `src/jail.rs` to:
- Optionally enable eBPF host blocking (new CLI flag: `--block-host`)
- After container creation, attach eBPF program to its cgroup
- Store eBPF blocker instance in jail state for cleanup

Example integration in `JailManager::create()`:
```rust
// After container creation
if config.block_host {
    let backend = PodmanBackend::new();
    let cgroup_path = backend.get_container_cgroup_path(&config.name).await?;
    let host_ips = get_host_ips()?;

    let mut blocker = EbpfHostBlocker::new();
    blocker.attach_to_cgroup(&cgroup_path, &host_ips).await?;

    // Store blocker for cleanup
    // self.ebpf_blocker = Some(blocker);
}
```

#### 6. Add Tests

- Unit tests for PID/cgroup path retrieval
- Integration tests for eBPF program loading
- End-to-end tests verifying connection blocking

#### 7. Handle Permissions

eBPF programs require elevated privileges:
- Loading eBPF programs typically requires `CAP_BPF` or `CAP_SYS_ADMIN`
- Attaching to cgroups may require `CAP_NET_ADMIN`

Options:
- Run jail-ai with sudo (not recommended for general use)
- Use systemd service with capabilities
- Check for CAP_BPF and provide clear error messages if missing

## References

- [Aya Book](https://aya-rs.dev/book/)
- [Linux eBPF Documentation](https://www.kernel.org/doc/html/latest/bpf/index.html)
- [BPF Program Types](https://docs.kernel.org/bpf/prog_cgroup_sockopt.html)
- [Cilium eBPF Guide](https://docs.cilium.io/en/stable/bpf/)

## Security Considerations

1. **Root privileges**: eBPF loading requires elevated permissions
2. **Kernel version**: Requires Linux kernel 4.10+ for cgroup sock_addr programs
3. **BPF verification**: The BPF verifier ensures programs are safe
4. **Performance**: eBPF programs run in kernel space with minimal overhead
5. **Bypass potential**: Container with CAP_SYS_ADMIN could potentially disable eBPF

## Alternative Approaches

If full eBPF implementation is too complex, consider:
1. **iptables/nftables rules**: Use host firewall rules (simpler but less granular)
2. **Network namespaces**: Use netns without any host connectivity
3. **Seccomp filters**: Block connect() syscall entirely (but affects all connections)
4. **SELinux/AppArmor**: Use LSM policies to restrict network access

## Next Steps

To continue implementation, choose one of these approaches:

### Option A: Full Aya Implementation (Pure Rust)
- Most modern and Rust-idiomatic
- Requires bpf-linker and more setup
- Best for long-term maintenance

### Option B: Simplified Implementation (Pre-compiled bytecode)
- Write eBPF program in C
- Compile separately and embed bytecode
- Load using Aya's bytecode loader
- Simpler build process

### Option C: Hybrid Approach (libbpf-rs)
- Use libbpf-rs instead of Aya
- Requires system libbpf library
- More mature ecosystem
- Better compatibility with existing BPF tools

Would you like me to proceed with one of these options, or would you prefer to provide specific guidance on the implementation approach?
