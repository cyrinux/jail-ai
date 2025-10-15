#!/bin/bash
set -x

echo "=== Test 1: Create container with eBPF ==="
./target/release/jail-ai create test-ebpf-lifecycle --block-host

echo ""
echo "=== Test 2: Check loader is running ==="
ps aux | grep jail-ai-ebpf-loader | grep -v grep

echo ""
echo "=== Test 3: Stop container ==="
podman stop jail__test-ebpf-lifecycle

echo ""
echo "=== Test 4: Check if loader is still running after container stop ==="
ps aux | grep jail-ai-ebpf-loader | grep -v grep || echo "Loader NOT running (expected)"

echo ""
echo "=== Test 5: Start container again ==="
podman start jail__test-ebpf-lifecycle

echo ""
echo "=== Test 6: Check loader after start ==="
ps aux | grep jail-ai-ebpf-loader | grep -v grep || echo "Loader NOT running"

echo ""
echo "=== Test 7: Exec into container (should trigger reattach) ==="
./target/release/jail-ai exec test-ebpf-lifecycle -- echo "test"

echo ""
echo "=== Test 8: Check loader after reattach ==="
ps aux | grep jail-ai-ebpf-loader | grep -v grep || echo "Loader NOT running - PROBLEM!"

echo ""
echo "=== Cleanup ==="
podman rm -f jail__test-ebpf-lifecycle
