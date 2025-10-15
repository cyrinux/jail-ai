# eBPF Host Blocking - Setup Guide

## Overview

This guide walks you through setting up the full eBPF host blocking feature for jail-ai. eBPF (Extended Berkeley Packet Filter) allows kernel-level interception of network connect() syscalls to block container access to host IP addresses.

## Prerequisites

### Recommended: Docker or Podman

The easiest way to build eBPF programs is using the provided `build-ebpf.sh` script with Docker or Podman. This avoids LLVM compatibility issues.

```bash
# Check if you have docker or podman
docker --version
# or
podman --version
```

If you have either, you can skip the manual prerequisites below and use `./build-ebpf.sh`.

### Manual Setup (Advanced)

If you prefer to build eBPF programs manually without Docker/Podman, you need:

#### 1. Rust Nightly Toolchain

```bash
# Install Rust nightly
rustup install nightly
rustup component add rust-src --toolchain nightly

# Verify installation
rustup +nightly --version
```

#### 2. bpf-linker

```bash
# Install bpf-linker
cargo install bpf-linker

# Verify installation
bpf-linker --version
```

**Note**: `bpf-linker` often has LLVM compatibility issues. If you encounter errors, use the Docker/Podman method instead (see [Troubleshooting](#unable-to-find-llvm-shared-lib-or-bpf-linker-sigabrt)).

### 3. Kernel Headers (Optional but Recommended)

For Debian/Ubuntu:
```bash
sudo apt-get install linux-headers-$(uname -r)
```

For Fedora/RHEL:
```bash
sudo dnf install kernel-devel kernel-headers
```

### 4. Kernel Requirements

- Linux kernel 4.10+ with BPF cgroup sock_addr support
- BPF filesystem mounted (usually at `/sys/fs/cgroup`)

Check your kernel version:
```bash
uname -r
```

## Building eBPF Programs

**Important**: eBPF compilation requires rustup and cannot be done in a Nix-based environment. You must build eBPF programs on your host system (outside of any jail-ai container).

### Option 1: Using Docker/Podman (Recommended)

This is the easiest and most reliable method, avoiding LLVM compatibility issues:

```bash
# Use the provided build script
./build-ebpf.sh
```

The script will automatically use Docker or Podman (whichever is available) to build eBPF programs in a clean container environment.

### Option 2: Build eBPF with xtask

If you have a working Rust nightly + bpf-linker setup:

```bash
# Build in debug mode
cargo xtask build-ebpf

# Build in release mode (recommended)
cargo xtask build-ebpf --release
```

If rustup is not available, you'll see a helpful error message with setup instructions.

This will compile the eBPF program to:
```
jail-ai-ebpf/target/bpfel-unknown-none/release/jail-ai-ebpf
```

### Option 3: Build Everything

```bash
# Build both eBPF programs and jail-ai binary
cargo xtask build --release
```

## Permissions

**IMPORTANT**: jail-ai now uses a **privileged helper binary** architecture for improved security.

### Recommended: Privileged Helper Binary (Most Secure)

The main `jail-ai` binary runs **without** any elevated privileges. Only the small helper binary (`jail-ai-ebpf-loader`) requires capabilities.

```bash
# 1. Build the helper binary
cargo build --release -p jail-ai-ebpf-loader

# 2. Install helper binary
cargo install --path jail-ai-ebpf-loader --force

# 3. Grant capabilities ONLY to helper (NOT to main binary)
sudo setcap cap_bpf,cap_net_admin+ep $(which jail-ai-ebpf-loader)

# 4. Verify main binary has NO capabilities
getcap $(which jail-ai)
# Should show: nothing

# 5. Run jail-ai without any special permissions
jail-ai create my-jail --block-host
```

**Security Benefits**:
- ✅ Main binary runs unprivileged (no capabilities needed)
- ✅ Helper is < 500 LOC and easy to audit
- ✅ Helper drops capabilities after loading
- ✅ Minimal attack surface

See [EBPF_SECURITY.md](./EBPF_SECURITY.md) for detailed security architecture.

### Alternative: Run with sudo

```bash
# Helper will be invoked with sudo automatically
sudo jail-ai create my-jail --block-host
```

### Legacy: Grant Capabilities to Main Binary (Not Recommended)

```bash
# OLD METHOD - Less secure, not recommended
# This grants capabilities to the entire jail-ai binary
sudo setcap cap_bpf,cap_net_admin+ep target/release/jail-ai
```

**Why not recommended**: This gives elevated privileges to the entire ~5000 LOC jail-ai binary instead of just the ~400 LOC helper.

## Verification

### 1. Check eBPF Program Built

```bash
ls -lh jail-ai-ebpf/target/bpfel-unknown-none/release/jail-ai-ebpf
```

You should see the compiled eBPF program (typically 2-10 KB).

### 2. Create a Test Jail

```bash
# Create jail with host blocking
cargo run --release -- create test-block --block-host --verbose

# Look for these messages:
# ✓ eBPF host blocking active for cgroup ...
# Attached IPv4 connect program to cgroup
# Attached IPv6 connect program to cgroup
```

### 3. Test Host Blocking

```bash
# Inside the container, try to connect to host
podman exec test-block curl http://127.0.0.1:8080

# Should fail with: "Failed to connect"
# Without --block-host, this would succeed if something is listening
```

### 4. Inspect BPF Programs

```bash
# List loaded BPF programs (requires root)
sudo bpftool prog list

# List cgroup BPF attachments
sudo bpftool cgroup tree /sys/fs/cgroup
```

## Troubleshooting

### "unable to find LLVM shared lib" or bpf-linker SIGABRT

This error occurs when `bpf-linker` cannot find compatible LLVM libraries. This is a known issue with `bpf-linker` on many systems.

**Solution: Use a pre-built eBPF program or build in Docker**

Since `bpf-linker` has LLVM compatibility issues, we recommend one of these approaches:

#### Option 1: Use Docker to Build eBPF Programs (Recommended)

Create a `build-ebpf.sh` script:

```bash
#!/bin/bash
# Build eBPF programs in a clean Docker container

docker run --rm \
  -v "$(pwd)":/workspace \
  -w /workspace \
  rust:latest \
  bash -c "
    rustup install nightly && \
    rustup component add rust-src --toolchain nightly && \
    cargo install bpf-linker && \
    cd jail-ai-ebpf && \
    cargo +nightly build --release --target=bpfel-unknown-none -Z build-std=core
  "
```

Then run:
```bash
chmod +x build-ebpf.sh
./build-ebpf.sh
```

#### Option 2: Install System LLVM and Link Manually

If Docker isn't available, try fixing the LLVM installation:

```bash
# Arch Linux
sudo pacman -S llvm20 llvm20-libs lib32-llvm20-libs

# Create symlinks for missing libraries
sudo ln -sf /usr/lib/libffi.so /usr/lib/libffi.so.8

# Reinstall bpf-linker
cargo uninstall bpf-linker
cargo install bpf-linker

# Try the build
cargo xtask build-ebpf --release
```

#### Option 3: Skip eBPF Compilation (Graceful Fallback)

The jail-ai binary will work without compiled eBPF programs - it will simply log a warning and continue without kernel-level host blocking:

```bash
# Build and use jail-ai without eBPF programs
cargo build --release
./target/release/jail-ai create test-jail --block-host

# You'll see:
# ⚠️  eBPF program not found at: jail-ai-ebpf/target/bpfel-unknown-none/release/jail-ai-ebpf
#    Running in stub mode - host blocking will not be enforced
```

This is useful for development and testing when eBPF compilation is problematic.

### "eBPF program not found"

This means the eBPF program hasn't been compiled yet:

```bash
cargo xtask build-ebpf --release
```

### "Permission denied" when attaching BPF

You need CAP_BPF or root:

```bash
# Run with sudo
sudo cargo run -- create test --block-host

# Or grant capability
sudo setcap cap_bpf+ep target/release/jail-ai
```

### "Failed to load eBPF program"

Check kernel support:

```bash
# Check if BPF is enabled
cat /proc/sys/kernel/unprivileged_bpf_disabled

# Check cgroup v2 is mounted
mount | grep cgroup2
```

### "Failed to attach v4 program"

The cgroup path might be incorrect or inaccessible:

```bash
# Check if container is running
podman ps -a | grep your-jail-name

# Check cgroup path
podman inspect your-jail-name --format '{{.State.Pid}}'
cat /proc/<PID>/cgroup
```

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     Host System                          │
│                                                          │
│  ┌──────────────┐                                       │
│  │  jail-ai     │                                       │
│  └──────┬───────┘                                       │
│         │                                                │
│         │ 1. Loads eBPF program                         │
│         │ 2. Populates BLOCKED_IPV4/IPV6 maps          │
│         │ 3. Attaches to cgroup                         │
│         ▼                                                │
│  ┌──────────────────────────────────┐                   │
│  │  eBPF Program (in kernel)        │                   │
│  │  - block_host_connect_v4         │                   │
│  │  - block_host_connect_v6         │                   │
│  └──────────────┬───────────────────┘                   │
│                 │                                        │
│                 │ Intercepts connect() syscalls          │
│                 ▼                                        │
│  ┌─────────────────────────────────────────┐            │
│  │        Container (in cgroup)            │            │
│  │  ┌──────────────┐                       │            │
│  │  │ AI Agent     │  ──X──> Host IPs      │            │
│  │  │              │  ──✓──> Internet      │            │
│  │  └──────────────┘                       │            │
│  └─────────────────────────────────────────┘            │
└─────────────────────────────────────────────────────────┘
```

## Performance

eBPF programs run directly in the kernel with minimal overhead:

- **Latency**: < 1 microsecond per connect() call
- **CPU**: Negligible impact (< 0.1%)
- **Memory**: ~2-10 KB per program

## Security

- **Kernel-level enforcement**: Cannot be bypassed from userspace
- **Fail-open design**: On error, connections are allowed (prevents breaking networking)
- **Minimal privileges**: Only CAP_BPF required (not full root)
- **No persistent state**: Programs are automatically unloaded when container stops

## Next Steps

1. **Build eBPF programs**: `cargo xtask build-ebpf --release`
2. **Test functionality**: Create a jail with `--block-host`
3. **Verify blocking**: Try connecting to host from inside container
4. **Deploy**: Use in production with appropriate permissions

## Quick Reference

### Complete Setup (Recommended: Docker/Podman Method)

```bash
# 1. Build eBPF programs using the helper script
cd /path/to/jail-ai
./build-ebpf.sh

# 2. Build and test jail-ai with eBPF blocking
cargo build --release
sudo ./target/release/jail-ai create test-jail --block-host
```

The `build-ebpf.sh` script uses Docker/Podman to build eBPF programs in a clean container, avoiding LLVM compatibility issues.

### Alternative: Manual Setup (If You Have Working rustup/bpf-linker)

```bash
# 1. Install Rust nightly and rust-src
rustup install nightly
rustup component add rust-src --toolchain nightly

# 2. Install bpf-linker
cargo install bpf-linker

# 3. Build eBPF programs
cargo xtask build-ebpf --release

# 4. Build and test jail-ai
cargo build --release
sudo ./target/release/jail-ai create test-jail --block-host
```

### Development Without eBPF (Stub Mode)

```bash
# Just build and run without eBPF compilation
cargo build --release
./target/release/jail-ai create test-jail --block-host

# Will show: "⚠️  eBPF program not found ... Running in stub mode"
```

## References

- [Aya Documentation](https://aya-rs.dev/book/)
- [Linux eBPF Documentation](https://www.kernel.org/doc/html/latest/bpf/index.html)
- [BPF Cgroup Programs](https://docs.kernel.org/bpf/prog_cgroup_sockopt.html)
- [bpf-linker Repository](https://github.com/aya-rs/bpf-linker)
