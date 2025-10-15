# jail-ai Troubleshooting Guide

## Podman Issues

### "iptables: Chain already exists" Error

**Error Message:**
```
error running container: setup network: netavark: code: 1, msg: iptables: Chain already exists.
```

**Cause:** Stale iptables chains from previous podman containers that weren't cleaned up properly.

**Solution:**

```bash
# Option 1: Reset podman network (safest)
podman network prune -f
podman system reset --force

# Option 2: Manual iptables cleanup (if you know what you're doing)
sudo iptables -t nat -F
sudo iptables -t filter -F
sudo iptables -t nat -X
sudo iptables -t filter -X

# Option 3: Restart networking
sudo systemctl restart netavark
sudo systemctl restart podman

# Then retry your jail-ai command
cargo run -- create test-jail
```

### "Permission denied" when creating containers

**Solution:**

```bash
# Ensure your user is in the right groups
sudo usermod -aG podman $USER
newgrp podman

# Or run with sudo
sudo jail-ai create test-jail
```

### Podman version too old

jail-ai requires podman 4.0+. Check your version:

```bash
podman --version

# If too old, upgrade:
# Arch Linux
sudo pacman -S podman

# Debian/Ubuntu (add podman repo first)
sudo apt-get update
sudo apt-get install podman
```

## eBPF Build Issues

See [EBPF_SETUP.md](./EBPF_SETUP.md) for detailed eBPF troubleshooting.

### Quick Summary

- **LLVM errors**: Use `./build-ebpf.sh` (Docker/Podman method)
- **No Docker/Podman**: Install system LLVM libraries or use stub mode
- **bpf-linker fails**: Don't use `--no-default-features`, just use Docker method

## Runtime Issues

### Container immediately exits

Check the logs:
```bash
podman logs jail__yourproject__hash__agent
```

### "Failed to attach eBPF program"

This is expected if:
- eBPF programs aren't compiled (stub mode will be used)
- You don't have CAP_BPF or root permissions

To run with eBPF blocking:
```bash
# Grant capability
sudo setcap cap_bpf+ep target/release/jail-ai

# Or run with sudo
sudo ./target/release/jail-ai create test --block-host
```

### "Cgroup not found"

Ensure cgroup v2 is mounted:
```bash
mount | grep cgroup2

# If not mounted:
sudo mount -t cgroup2 none /sys/fs/cgroup
```

## Development Issues

### Workspace build fails

```bash
# Clean build
cargo clean
cargo build

# If still fails, check workspace members:
ls -d xtask jail-ai-ebpf
```

### Tests fail with "block_host field missing"

The `block_host` field was added to `JailConfig`. Update any test code that creates `JailConfig` structs to include:
```rust
block_host: false,
```

## Getting Help

1. Check logs: `podman logs <container-id>`
2. Enable debug logging: `RUST_LOG=debug jail-ai create test`
3. Verify podman works: `podman run --rm alpine echo "test"`
4. Check kernel version: `uname -r` (needs 4.10+ for eBPF)
5. Report issues: https://github.com/cyrinux/jail-ai/issues
