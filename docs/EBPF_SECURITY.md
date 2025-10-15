# eBPF Security Architecture - Privileged Helper Binary

## Overview

jail-ai uses a **privileged helper binary architecture** to load eBPF programs, significantly reducing the security risk compared to granting capabilities to the main binary.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         Host System                          │
│                                                              │
│  ┌──────────────────┐         ┌────────────────────────┐   │
│  │    jail-ai       │  JSON   │ jail-ai-ebpf-loader    │   │
│  │ (unprivileged)   │ ──────> │ (privileged helper)    │   │
│  │                  │  stdin  │                        │   │
│  │ NO CAPABILITIES  │ <────── │ CAP_BPF + CAP_NET_ADMIN│   │
│  └──────────────────┘  stdout └───────────┬────────────┘   │
│                                            │                 │
│                                            │ Load & attach   │
│                                            ▼                 │
│                              ┌──────────────────────────┐   │
│                              │  eBPF Program (kernel)   │   │
│                              │  - Validates inputs      │   │
│                              │  - Loads program         │   │
│                              │  - Populates maps        │   │
│                              │  - Attaches to cgroup    │   │
│                              │  - Drops capabilities    │   │
│                              │  - Exits immediately     │   │
│                              └───────────┬──────────────┘   │
│                                          │                   │
│                                          │ Filters traffic   │
│                                          ▼                   │
│                          ┌────────────────────────────────┐ │
│                          │   Container (in cgroup)        │ │
│                          │  ┌─────────────┐               │ │
│                          │  │  AI Agent   │ ──X─> Host    │ │
│                          │  │             │ ──✓─> Internet│ │
│                          │  └─────────────┘               │ │
│                          └────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Security Benefits

### 1. Minimal Attack Surface

**Main Binary (`jail-ai`)**:
- ✅ Runs **without** any elevated privileges
- ✅ No CAP_BPF or CAP_NET_ADMIN capabilities needed
- ✅ Can be run by unprivileged users
- ✅ Larger codebase but no privileged operations

**Helper Binary (`jail-ai-ebpf-loader`)**:
- ✅ **< 500 lines of code** (easy to audit)
- ✅ Stateless: exits immediately after loading
- ✅ No network access
- ✅ No file writes (except BPF operations)
- ✅ Rigorous input validation
- ✅ Drops capabilities after loading

### 2. Privilege Separation

| Component | Privileges | Code Size | Network | File Access |
|-----------|-----------|-----------|---------|-------------|
| jail-ai | None | ~5000 LOC | Yes | Read/Write |
| jail-ai-ebpf-loader | CAP_BPF, CAP_NET_ADMIN | ~400 LOC | No | BPF only |

### 3. Input Validation

The helper binary validates all inputs before performing any privileged operations:

```rust
fn validate_request(request: &LoadRequest) -> Result<(), String> {
    // ✓ cgroup_path must start with /sys/fs/cgroup
    // ✓ No path traversal (../ or //)
    // ✓ Path must exist
    // ✓ IP list must be non-empty and < 1000 entries
    // ✓ All IPs must be valid
}
```

### 4. Capability Dropping

After loading eBPF programs, the helper immediately drops all capabilities:

```rust
fn drop_capabilities() -> Result<(), String> {
    caps::clear(None, caps::CapSet::Effective)?;
    caps::clear(None, caps::CapSet::Permitted)?;
    caps::clear(None, caps::CapSet::Inheritable)?;
    // Now running as unprivileged process
}
```

## Installation & Setup

### Step 1: Build eBPF Programs

```bash
# Build eBPF programs using Docker/Podman (recommended)
./build-ebpf.sh

# Or manually if you have rustup + bpf-linker
cargo xtask build-ebpf --release
```

### Step 2: Build Helper Binary

```bash
# Build the helper binary
cargo build --release -p jail-ai-ebpf-loader

# Install to system (optional)
cargo install --path jail-ai-ebpf-loader --force
```

### Step 3: Grant Capabilities to Helper ONLY

```bash
# Grant capabilities ONLY to the helper binary
sudo setcap cap_bpf,cap_net_admin+ep $(which jail-ai-ebpf-loader)

# Verify capabilities
getcap $(which jail-ai-ebpf-loader)
# Should show: cap_bpf,cap_net_admin+ep
```

### Step 4: Verify jail-ai Has No Capabilities

```bash
# Check that main binary has NO capabilities
getcap $(which jail-ai)
# Should show: nothing (no capabilities)

# Verify it works without privileges
jail-ai create test-jail --block-host
```

## Comparison with Previous Architecture

### Before (Capabilities on Main Binary)

```bash
# OLD METHOD - LESS SECURE
sudo setcap cap_bpf,cap_net_admin+ep $(which jail-ai)

# Risk: Entire jail-ai binary runs with elevated privileges
# Attack surface: ~5000 LOC with network and file access
```

### After (Privileged Helper Binary)

```bash
# NEW METHOD - MORE SECURE
sudo setcap cap_bpf,cap_net_admin+ep $(which jail-ai-ebpf-loader)

# Risk: Only small helper runs with elevated privileges
# Attack surface: ~400 LOC, stateless, no network/file access
# Main binary: completely unprivileged
```

## Security Audit Checklist

When auditing the helper binary, verify:

- [ ] Binary is < 500 LOC and easy to understand
- [ ] No network operations (no sockets, no DNS, no HTTP)
- [ ] No file writes except BPF syscalls
- [ ] All inputs are validated before privileged operations
- [ ] cgroup paths are sanitized (no path traversal)
- [ ] IP addresses are validated
- [ ] Capabilities are dropped after eBPF loading
- [ ] Process exits immediately after loading
- [ ] No persistent state or background operations

## Alternative Security Measures

If you don't want to use file capabilities at all, consider:

### Option 1: Sudo Wrapper (Most Restrictive)

```bash
# /etc/sudoers.d/jail-ai-loader
yourusername ALL=(root) NOPASSWD: /usr/local/bin/jail-ai-ebpf-loader

# Advantages:
# - Full audit logging via sudo
# - Can restrict to specific users
# - No file capabilities needed
```

### Option 2: Systemd Ambient Capabilities

```ini
# /etc/systemd/system/jail-ai-loader.socket
[Unit]
Description=jail-ai eBPF Loader Socket

[Socket]
ListenStream=/run/jail-ai-loader.sock
Accept=yes

[Install]
WantedBy=sockets.target

# /etc/systemd/system/jail-ai-loader@.service
[Service]
ExecStart=/usr/local/bin/jail-ai-ebpf-loader
StandardInput=socket
AmbientCapabilities=CAP_BPF CAP_NET_ADMIN
```

### Option 3: SELinux/AppArmor Profiles

Confine the helper binary with mandatory access control:

```bash
# SELinux example (conceptual)
allow jail_ai_loader_t bpf_t:bpf { prog_load map_create };
allow jail_ai_loader_t cgroup_t:dir { open read };
# Deny everything else
```

## Troubleshooting

### Helper Not Found

```
Error: jail-ai-ebpf-loader not found
```

**Solution**: Install the helper binary:
```bash
cargo install --path jail-ai-ebpf-loader --force
```

### Permission Denied

```
Error: Failed to load eBPF: Permission denied
```

**Solution**: Grant capabilities to helper:
```bash
sudo setcap cap_bpf,cap_net_admin+ep $(which jail-ai-ebpf-loader)
```

### Capability Verification

```bash
# Check helper has capabilities
getcap $(which jail-ai-ebpf-loader)
# Expected: cap_bpf,cap_net_admin+ep

# Check main binary has NO capabilities
getcap $(which jail-ai)
# Expected: nothing (empty output)
```

## Performance

The helper binary adds minimal overhead:

- **Startup**: < 50ms (one-time per container creation)
- **Memory**: ~2 MB (process exits immediately)
- **CPU**: Negligible (< 0.1% during loading)

## Future Improvements

Potential enhancements for even better security:

1. **Socket Activation**: Use systemd socket activation to avoid file capabilities entirely
2. **D-Bus Interface**: Use D-Bus for IPC with PolicyKit authorization
3. **Seccomp Filters**: Add seccomp-bpf to restrict helper syscalls
4. **Landlock**: Use Landlock LSM to restrict file access
5. **User Namespaces**: Explore using user namespaces for privilege separation

## References

- [Linux Capabilities](https://man7.org/linux/man-pages/man7/capabilities.7.html)
- [eBPF Security](https://ebpf.io/what-is-ebpf/#security)
- [Privilege Separation](https://en.wikipedia.org/wiki/Privilege_separation)
- [setcap(8)](https://man7.org/linux/man-pages/man8/setcap.8.html)
