#!/bin/bash
# Debug script for eBPF host blocking

echo "=== eBPF Programs Loaded ==="
sudo bpftool prog list | grep -A 5 "cgroup_skb"

echo ""
echo "=== eBPF Maps ==="
sudo bpftool map list | grep -i blocked

echo ""
echo "=== BLOCKED_IPV4 Map Contents ==="
# Find the map ID for BLOCKED_IPV4
MAP_ID=$(sudo bpftool map list | grep BLOCKED_IPV4 | head -1 | awk '{print $1}' | tr -d ':')
if [ -n "$MAP_ID" ]; then
    echo "Map ID: $MAP_ID"
    echo "Dumping map contents (first 20 entries):"
    sudo bpftool map dump id "$MAP_ID" | head -40
    
    echo ""
    echo "Looking for 169.254.1.2 (0xa9fe0102 in network byte order):"
    # 169.254.1.2 = a9 fe 01 02 in network byte order
    sudo bpftool map dump id "$MAP_ID" | grep -i "a9 fe 01 02\|02 01 fe a9"
else
    echo "BLOCKED_IPV4 map not found!"
fi

echo ""
echo "=== Check if program is attached to cgroups ==="
# Get container name from jail-ai
CONTAINER_NAME=$(podman ps --format '{{.Names}}' | grep jail__jail-ai || echo "")
if [ -n "$CONTAINER_NAME" ]; then
    echo "Container: $CONTAINER_NAME"
    CGROUP_PATH=$(podman inspect "$CONTAINER_NAME" --format '{{.State.Pid}}' | xargs -I {} cat /proc/{}/cgroup | head -1 | cut -d: -f3)
    echo "Cgroup: $CGROUP_PATH"
    
    if [ -d "/sys/fs/cgroup$CGROUP_PATH" ]; then
        echo "Attached programs:"
        sudo bpftool cgroup show "/sys/fs/cgroup$CGROUP_PATH" 2>/dev/null || echo "No programs attached or permission denied"
    fi
else
    echo "No jail-ai container found"
fi

echo ""
echo "=== Test: What is 169.254.1.2 in different byte orders? ==="
echo "Network byte order (big-endian): 0xa9fe0102"
echo "Host byte order (little-endian on x86): 0x0201fea9"
echo "As bytes: a9 fe 01 02"
