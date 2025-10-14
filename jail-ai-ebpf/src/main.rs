#![no_std]
#![no_main]

use aya_ebpf::{
    macros::{cgroup_skb, map, tracepoint},
    maps::{HashMap, PerCpuArray, PerfEventArray},
    programs::{SkBuffContext, TracePointContext},
    EbpfContext,
};

/// Event structure for exec syscalls
/// This structure is sent from kernel to userspace via perf buffer
#[repr(C)]
pub struct ExecEvent {
    /// Process ID
    pub pid: u32,
    /// Parent process ID
    pub ppid: u32,
    /// User ID
    pub uid: u32,
    /// Timestamp (nanoseconds)
    pub timestamp: u64,
    /// Command name (truncated to 16 bytes)
    pub comm: [u8; 16],
    /// Filename being executed (truncated to 256 bytes)
    pub filename: [u8; 256],
}

/// Map storing blocked IPv4 addresses
/// Key: u32 (IPv4 address in network byte order)
/// Value: u8 (unused, just for existence check)
/// Note: Increased from 256 to 1024 to handle systems with many network interfaces
#[map]
static BLOCKED_IPV4: HashMap<u32, u8> = HashMap::with_max_entries(1024, 0);

/// Map storing blocked IPv6 addresses
/// Key: [u32; 4] (IPv6 address as 4 u32s in network byte order)
/// Value: u8 (unused, just for existence check)
/// Note: Increased from 256 to 1024 to handle systems with many network interfaces
#[map]
static BLOCKED_IPV6: HashMap<[u32; 4], u8> = HashMap::with_max_entries(1024, 0);

/// Perf event array for sending exec events to userspace
#[map]
static EXEC_EVENTS: PerfEventArray<ExecEvent> = PerfEventArray::new(0);

/// Per-CPU array for temporary ExecEvent storage (avoids stack limit)
/// Using a single-entry array accessed by CPU ID to store events during construction
#[map]
static EXEC_EVENT_STORAGE: PerCpuArray<ExecEvent> =
    PerCpuArray::with_max_entries(1, 0);

// IPv4 header offsets (no Ethernet header in cgroup_skb)
const IPV4_DST_OFFSET: usize = 16; // Destination address at byte 16 in IP header

// IPv6 header offsets (no Ethernet header in cgroup_skb)
const IPV6_DST_OFFSET: usize = 24; // Destination address at byte 24 in IPv6 header

/// Hook for egress (outgoing) packets
///
/// This program is attached to BPF_CGROUP_INET_EGRESS and inspects
/// all outgoing packets from the container.
///
/// Note: In cgroup_skb context, packets start at the IP layer (no Ethernet header)
///
/// Returns:
/// - 1 (pass) if the destination IP is not in the blocked list
/// - 0 (drop) if the destination IP is blocked
#[cgroup_skb(egress)]
pub fn block_host_egress(ctx: SkBuffContext) -> i32 {
    match try_block_host_egress(&ctx) {
        Ok(ret) => ret,
        Err(_) => 1, // On error, allow the packet (fail-open)
    }
}

fn try_block_host_egress(ctx: &SkBuffContext) -> Result<i32, ()> {
    // In cgroup_skb, packets start at IP header (no Ethernet header)
    // Read first byte to determine IP version from version nibble
    let version_byte: u8 = ctx.load(0).map_err(|_| ())?;
    let ip_version = (version_byte >> 4) & 0x0F;

    match ip_version {
        4 => {
            // IPv4 packet
            try_block_ipv4(ctx)
        }
        6 => {
            // IPv6 packet
            try_block_ipv6(ctx)
        }
        _ => {
            // Unknown IP version - allow
            Ok(1)
        }
    }
}

fn try_block_ipv4(ctx: &SkBuffContext) -> Result<i32, ()> {
    // Read destination IP from IPv4 header
    // In cgroup_skb, packet starts at IP header (byte 0)
    // Destination IP is at offset 16 within IP header

    // Read the 4 bytes of the IP address individually to ensure correct byte order
    let byte0: u8 = ctx.load(IPV4_DST_OFFSET).map_err(|_| ())?;
    let byte1: u8 = ctx.load(IPV4_DST_OFFSET + 1).map_err(|_| ())?;
    let byte2: u8 = ctx.load(IPV4_DST_OFFSET + 2).map_err(|_| ())?;
    let byte3: u8 = ctx.load(IPV4_DST_OFFSET + 3).map_err(|_| ())?;

    // Construct u32 in network byte order (big-endian)
    let dst_ip: u32 =
        ((byte0 as u32) << 24) | ((byte1 as u32) << 16) | ((byte2 as u32) << 8) | (byte3 as u32);

    // Allow localhost traffic (127.0.0.0/8)
    // Check if first byte is 127
    if byte0 == 127 {
        return Ok(1);
    }

    // Check if this IP is in the blocked list
    unsafe {
        if BLOCKED_IPV4.get(&dst_ip).is_some() {
            // IP is blocked, drop the packet
            return Ok(0);
        }
    }

    // IP is not blocked, allow the packet
    Ok(1)
}

fn try_block_ipv6(ctx: &SkBuffContext) -> Result<i32, ()> {
    // Read destination IPv6 address from IPv6 header
    // In cgroup_skb, packet starts at IPv6 header (byte 0)
    // Destination IP is at offset 24 within IPv6 header (16 bytes total)

    // Optimize for localhost (::1) by checking first u32 early
    // ::1 is represented as [0, 0, 0, 1] in network byte order
    // Load first u32 to check if it might be localhost
    let first: u32 = ctx.load(IPV6_DST_OFFSET).map_err(|_| ())?;

    // If first u32 is non-zero, it's not localhost - load rest and check map
    if first != 0 {
        // Load remaining u32s
        let dst_ip: [u32; 4] = [
            first,
            ctx.load(IPV6_DST_OFFSET + 4).map_err(|_| ())?,
            ctx.load(IPV6_DST_OFFSET + 8).map_err(|_| ())?,
            ctx.load(IPV6_DST_OFFSET + 12).map_err(|_| ())?,
        ];

        // Check if this IP is in the blocked list
        unsafe {
            if BLOCKED_IPV6.get(&dst_ip).is_some() {
                // IP is blocked, drop the packet
                return Ok(0);
            }
        }

        // IP is not blocked, allow the packet
        return Ok(1);
    }

    // First u32 is 0, might be localhost - check remaining u32s
    let second: u32 = ctx.load(IPV6_DST_OFFSET + 4).map_err(|_| ())?;
    let third: u32 = ctx.load(IPV6_DST_OFFSET + 8).map_err(|_| ())?;
    let fourth: u32 = ctx.load(IPV6_DST_OFFSET + 12).map_err(|_| ())?;

    // Check for ::1 (localhost)
    if second == 0 && third == 0 && fourth == 1 {
        return Ok(1); // Allow localhost
    }

    // Not localhost, check if blocked
    let dst_ip: [u32; 4] = [first, second, third, fourth];
    unsafe {
        if BLOCKED_IPV6.get(&dst_ip).is_some() {
            return Ok(0); // Blocked
        }
    }

    // IP is not blocked, allow the packet
    Ok(1)
}

/// Tracepoint for sched_process_exec
///
/// This tracepoint is triggered whenever a process executes a new program.
/// It captures the process information and sends it to userspace via perf buffer.
///
/// Tracepoint: /sys/kernel/debug/tracing/events/sched/sched_process_exec
#[tracepoint(category = "sched", name = "sched_process_exec")]
pub fn trace_exec(ctx: TracePointContext) -> u32 {
    match try_trace_exec(&ctx) {
        Ok(_) => 0,
        Err(_) => 1,
    }
}

fn try_trace_exec(ctx: &TracePointContext) -> Result<(), ()> {
    // Get a pointer to the per-CPU event storage (avoids stack overflow)
    let event = EXEC_EVENT_STORAGE
        .get_ptr_mut(0)
        .ok_or(())?;

    // Read process information using eBPF helpers
    let pid = ctx.pid(); // Get PID as u32
    let uid = ctx.uid();

    // Get timestamp in nanoseconds
    let timestamp = unsafe { aya_ebpf::helpers::bpf_ktime_get_ns() };

    // Initialize event fields
    unsafe {
        (*event).pid = pid;
        (*event).ppid = 0; // We'll get this in userspace if needed
        (*event).uid = uid;
        (*event).timestamp = timestamp;
    }

    // Get command name and write to event
    let comm = aya_ebpf::helpers::bpf_get_current_comm()
        .unwrap_or([0u8; 16]);
    unsafe {
        (*event).comm = comm;
    }

    // Read filename from tracepoint args
    // The filename is at a specific offset in the tracepoint context
    // For sched_process_exec, the filename is passed as an argument
    // Try to read the filename pointer from tracepoint args
    // offset 16 bytes into the tracepoint args is the filename pointer
    let filename_ptr: *const u8 = unsafe {
        ctx.read_at::<u64>(16).map_err(|_| ())? as *const u8
    };

    // Initialize filename to zeros
    unsafe {
        (*event).filename = [0u8; 256];
    }

    // Read the filename string from the pointer
    if !filename_ptr.is_null() {
        unsafe {
            let _ = aya_ebpf::helpers::bpf_probe_read_kernel_str_bytes(
                filename_ptr as *const u8,
                &mut (*event).filename,
            );
        }
    }

    // Send event to userspace via perf buffer
    unsafe {
        EXEC_EVENTS.output(ctx, &*event, 0);
    }

    Ok(())
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
