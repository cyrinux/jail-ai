# eBPF Helper Binary Migration Guide

## Summary

jail-ai has been refactored to use a **privileged helper binary** architecture for loading eBPF programs. This significantly improves security by eliminating the need to grant capabilities to the main jail-ai binary.

## What Changed

### Before (v0.43.x and earlier)

```bash
# OLD: Capabilities on main binary (LESS SECURE)
cargo build --release
sudo setcap cap_bpf,cap_net_admin+ep target/release/jail-ai
./target/release/jail-ai create test --block-host
```

**Risk**: Entire ~5000 LOC jail-ai binary runs with elevated privileges

### After (v0.44.0+)

```bash
# NEW: Capabilities only on small helper (MORE SECURE)
./build-ebpf.sh                                    # Build eBPF programs
cargo build --release                               # Build main binary (no caps!)
cargo build --release -p jail-ai-ebpf-loader       # Build helper
cargo install --path jail-ai-ebpf-loader --force   # Install helper
sudo setcap cap_bpf,cap_net_admin+ep $(which jail-ai-ebpf-loader)
jail-ai create test --block-host                    # No sudo needed!
```

**Benefit**: Only ~400 LOC helper binary has elevated privileges; main binary is completely unprivileged

## Architecture

```
┌─────────────────────────────────────────────────┐
│  jail-ai (UNPRIVILEGED)                         │
│  - No capabilities needed                       │
│  - ~5000 LOC                                    │
│  - Full network and file access                 │
│  - Can be run by any user                       │
└─────────┬───────────────────────────────────────┘
          │ JSON over stdin/stdout
          │
┌─────────▼───────────────────────────────────────┐
│  jail-ai-ebpf-loader (PRIVILEGED HELPER)        │
│  - CAP_BPF + CAP_NET_ADMIN only                 │
│  - ~400 LOC (easy to audit)                     │
│  - No network access                            │
│  - No file writes (except BPF ops)              │
│  - Validates all inputs                         │
│  - Drops capabilities after loading             │
│  - Exits immediately (stateless)                │
└─────────┬───────────────────────────────────────┘
          │ Loads eBPF into kernel
          │
┌─────────▼───────────────────────────────────────┐
│  eBPF Program (Kernel)                          │
│  - Blocks container → host connections          │
│  - Auto-detached when container stops           │
└─────────────────────────────────────────────────┘
```

## Migration Steps

### For Developers

1. **Update build process**:
   ```bash
   # Build eBPF programs (one-time, or when eBPF code changes)
   ./build-ebpf.sh

   # Build both binaries
   make build-all

   # Or manually:
   cargo build --release                       # Main binary
   cargo build --release -p jail-ai-ebpf-loader  # Helper
   ```

2. **Install and configure helper**:
   ```bash
   # Install helper binary
   cargo install --path jail-ai-ebpf-loader --force

   # Grant capabilities to helper ONLY
   sudo setcap cap_bpf,cap_net_admin+ep $(which jail-ai-ebpf-loader)

   # Verify main binary has NO capabilities
   getcap $(which jail-ai)
   # Should output: nothing
   ```

3. **Remove old capabilities** (if you had them):
   ```bash
   # Remove capabilities from main binary if you had them before
   sudo setcap -r $(which jail-ai) 2>/dev/null || true
   ```

### For Users (Installing from Source)

```bash
# Clone and build
git clone https://github.com/cyrinux/jail-ai.git
cd jail-ai

# Build eBPF programs
./build-ebpf.sh

# Build everything (or use: make build-all install-loader)
cargo build --release
cargo install --path . --force
cargo install --path jail-ai-ebpf-loader --force

# Grant capabilities ONLY to helper
sudo setcap cap_bpf,cap_net_admin+ep $(which jail-ai-ebpf-loader)

# Verify security
echo "Main binary capabilities: $(getcap $(which jail-ai) 2>/dev/null || echo 'none ✓')"
echo "Helper capabilities: $(getcap $(which jail-ai-ebpf-loader))"

# Test
jail-ai create test-jail --block-host
```

### For Package Maintainers

Update your packaging scripts:

```bash
# Build step
./build-ebpf.sh
cargo build --release
cargo build --release -p jail-ai-ebpf-loader

# Install step
install -m 755 target/release/jail-ai /usr/local/bin/
install -m 755 target/release/jail-ai-ebpf-loader /usr/local/bin/

# Post-install (in postinst script)
setcap cap_bpf,cap_net_admin+ep /usr/local/bin/jail-ai-ebpf-loader
```

**Important for packages**: Do NOT set capabilities on `/usr/local/bin/jail-ai`, only on the helper.

## Verification

Check that the migration was successful:

```bash
# 1. Helper should have capabilities
getcap $(which jail-ai-ebpf-loader)
# Expected: cap_bpf,cap_net_admin+ep

# 2. Main binary should have NO capabilities
getcap $(which jail-ai)
# Expected: empty output or "No capabilities"

# 3. Test that it works without sudo
jail-ai create test-jail --block-host --verbose

# Should see:
# ✓ Loading eBPF program via helper binary...
# ✓ eBPF host blocking active for cgroup ...
```

## Troubleshooting

### "jail-ai-ebpf-loader not found"

**Cause**: Helper binary not installed

**Solution**:
```bash
cargo install --path jail-ai-ebpf-loader --force
```

### "Permission denied" when loading eBPF

**Cause**: Helper binary doesn't have capabilities

**Solution**:
```bash
sudo setcap cap_bpf,cap_net_admin+ep $(which jail-ai-ebpf-loader)
```

### "Failed to spawn loader binary"

**Cause**: Helper binary is not in PATH

**Solution**: jail-ai searches for the helper in:
1. Same directory as jail-ai binary
2. PATH directories
3. `/usr/local/bin`
4. `/usr/bin`

Make sure the helper is in one of these locations.

### Still being asked for sudo

**Cause**: Old capabilities still on main binary

**Solution**:
```bash
# Remove old capabilities from main binary
sudo setcap -r $(which jail-ai)

# Verify
getcap $(which jail-ai)
# Should show nothing
```

## Security Benefits

1. **Reduced Attack Surface**:
   - Before: ~5000 LOC with elevated privileges
   - After: ~400 LOC with elevated privileges

2. **Privilege Separation**:
   - Main binary: unprivileged, full features
   - Helper: privileged, minimal, auditable

3. **Input Validation**:
   - Helper rigorously validates all inputs
   - Prevents path traversal, malicious IPs, etc.

4. **Capability Dropping**:
   - Helper drops ALL capabilities after loading eBPF
   - Runs as unprivileged process before exiting

5. **Audit Trail**:
   - Easy to audit ~400 LOC helper binary
   - Stateless: no persistent state or background processes

## For More Information

- [EBPF_SECURITY.md](./EBPF_SECURITY.md) - Detailed security architecture
- [EBPF_SETUP.md](./EBPF_SETUP.md) - Setup and troubleshooting guide
- [jail-ai-ebpf-loader source](/workspace/jail-ai-ebpf-loader/src/main.rs) - Helper binary source code

## Backwards Compatibility

The old method of setting capabilities on the main binary still works but is **not recommended**:

```bash
# LEGACY (NOT RECOMMENDED)
sudo setcap cap_bpf,cap_net_admin+ep $(which jail-ai)
```

This will bypass the helper binary and load eBPF directly from the main process, but sacrifices the security benefits.

## FAQ

**Q: Do I need to rebuild eBPF programs after upgrading?**
A: No, unless the eBPF program code itself changed. The helper binary just loads the existing eBPF program.

**Q: Can I use the helper with setuid instead of capabilities?**
A: Not recommended. Use capabilities or sudo wrapper instead.

**Q: Does this work on all Linux distributions?**
A: Yes, requires Linux kernel 4.10+ with BPF support (same as before).

**Q: What if I don't want to use eBPF host blocking?**
A: Simply don't use the `--block-host` flag. The helper is only invoked when needed.

**Q: Can I run jail-ai completely unprivileged now?**
A: Yes! The main jail-ai binary requires no special permissions. Only the helper (which is only called when using `--block-host`) needs capabilities.
