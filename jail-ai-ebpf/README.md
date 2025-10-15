# jail-ai eBPF Host Blocker

This directory contains the eBPF programs that provide kernel-level network filtering to block container connections to host IP addresses.

## What It Does

The eBPF programs intercept `connect()` system calls from containers and block connections to:
- Localhost (127.0.0.1, ::1)
- Host network interfaces
- Cloud metadata services (169.254.169.254, 10.0.2.2)

This provides defense-in-depth security by enforcing network isolation at the kernel level, which cannot be bypassed from userspace.

## Building

### Easy Way (Docker/Podman)

From the root of the jail-ai repository:

```bash
./build-ebpf.sh
```

This builds the eBPF programs in a clean container, avoiding LLVM compatibility issues.

### Manual Way

If you have Rust nightly and bpf-linker installed:

```bash
cd jail-ai-ebpf
cargo +nightly build --release --target=bpfel-unknown-none -Z build-std=core
```

Or use the xtask from the root:

```bash
cargo xtask build-ebpf --release
```

## Architecture

```
┌─────────────────────────────────────────┐
│          Container Process              │
│                                         │
│    connect(127.0.0.1:8080) ───┐        │
└────────────────────────────────┼────────┘
                                 │
                    ┌────────────▼────────────┐
                    │  eBPF Program (Kernel)  │
                    │  - block_host_connect_v4│
                    │  - block_host_connect_v6│
                    │  - BLOCKED_IPV4 map     │
                    │  - BLOCKED_IPV6 map     │
                    └────────────┬────────────┘
                                 │
                    ┌────────────▼────────────┐
                    │  Decision:              │
                    │  • Host IP? → DENY (0)  │
                    │  • Other?   → ALLOW (1) │
                    └─────────────────────────┘
```

## Files

- `src/main.rs` - eBPF program with cgroup connect hooks
- `Cargo.toml` - eBPF-specific dependencies (aya-ebpf)

## Output

The compiled eBPF program is written to:
```
target/bpfel-unknown-none/release/jail-ai-ebpf
```

This binary is loaded by the main jail-ai program when using `--block-host` flag.

## Troubleshooting

See [../docs/EBPF_SETUP.md](../docs/EBPF_SETUP.md) for detailed troubleshooting, especially the section on LLVM errors.

## References

- [Aya Book](https://aya-rs.dev/book/) - Aya eBPF library documentation
- [BPF Cgroup Programs](https://docs.kernel.org/bpf/prog_cgroup_sockopt.html) - Linux kernel docs
