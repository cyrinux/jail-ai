#!/bin/bash
# Build eBPF programs in a clean Docker/Podman container
# This avoids LLVM compatibility issues on the host system

set -e

CONTAINER_CMD="docker"

# Check if podman is available (prefer podman if present)
if command -v podman &> /dev/null; then
    CONTAINER_CMD="podman"
elif ! command -v docker &> /dev/null; then
    echo "Error: Neither docker nor podman is available"
    echo "Please install one of them to build eBPF programs"
    exit 1
fi

echo "Using $CONTAINER_CMD to build eBPF programs..."

CONTAINER_NAME="build-ebpf"

# Check if container exists
if $CONTAINER_CMD container exists "$CONTAINER_NAME" 2>/dev/null; then
    echo "Reusing existing container '$CONTAINER_NAME'..."
    
    # Check if container is running
    CONTAINER_STATE=$($CONTAINER_CMD inspect -f '{{.State.Status}}' "$CONTAINER_NAME")
    
    if [ "$CONTAINER_STATE" != "running" ]; then
        echo "Starting stopped container..."
        $CONTAINER_CMD start "$CONTAINER_NAME"
    fi
    
    # Execute build in existing container
    $CONTAINER_CMD exec -w /workspace "$CONTAINER_NAME" bash -c "
        set -e
        echo '==> Building eBPF programs...'
        cd jail-ai-ebpf
        cargo +nightly build --release --target=bpfel-unknown-none -Z build-std=core

        echo ''
        echo '✓ eBPF programs built successfully!'
        echo '  Location: target/bpfel-unknown-none/release/jail-ai-ebpf'
    "
else
    echo "Creating new container '$CONTAINER_NAME'..."
    
    # Create new container with tools installed
    $CONTAINER_CMD run -d --name "$CONTAINER_NAME" \
      -v "$(pwd)":/workspace \
      -w /workspace \
      rust:latest \
      sleep infinity
    
    # Install tools in the new container
    $CONTAINER_CMD exec "$CONTAINER_NAME" bash -c "
        set -e
        echo '==> Installing Rust nightly...'
        rustup install nightly
        rustup component add rust-src --toolchain nightly

        echo '==> Installing bpf-linker...'
        cargo install bpf-linker
    "
    
    # Build eBPF programs
    $CONTAINER_CMD exec -w /workspace "$CONTAINER_NAME" bash -c "
        set -e
        echo '==> Building eBPF programs...'
        cd jail-ai-ebpf
        cargo +nightly build --release --target=bpfel-unknown-none -Z build-std=core

        echo ''
        echo '✓ eBPF programs built successfully!'
        echo '  Location: target/bpfel-unknown-none/release/jail-ai-ebpf'
    "
fi

echo ""
echo "Build complete! You can now run:"
echo "  cargo build --release"
echo "  sudo setcap cap_bpf,cap_net_admin+ep ./target/release/jail-ai "
echo "  ./target/release/jail-ai create test-jail --block-host"

