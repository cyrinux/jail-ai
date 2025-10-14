use crate::error::{JailError, Result};
use aya::{
    maps::perf::AsyncPerfEventArray,
    programs::TracePoint,
    util::online_cpus,
    Bpf,
};
use bytes::BytesMut;
use std::fs;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

#[cfg(debug_assertions)]
use tracing::warn as debug_warn;

/// Event structure for exec syscalls (must match the eBPF program)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ExecEvent {
    pub pid: u32,
    pub ppid: u32,
    pub uid: u32,
    pub timestamp: u64,
    pub comm: [u8; 16],
    pub filename: [u8; 256],
}

/// eBPF-based exec monitor for containers
///
/// This struct manages eBPF programs that trace exec syscalls from containers.
/// It attaches eBPF tracepoints to capture process execution events.
///
/// # Requirements
/// - CAP_BPF or root privileges to load eBPF programs
/// - Linux kernel 4.7+ with BPF tracepoint support
///
/// # Implementation
/// - **Release builds**: eBPF bytecode is embedded in the binary at compile time
/// - **Debug builds**: eBPF program is loaded from file for easier development
///
/// # Usage
/// ```no_run
/// # use jail_ai::ebpf::monitor::ExecMonitor;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut monitor = ExecMonitor::new();
/// monitor.attach().await?;
/// # Ok(())
/// # }
/// ```
pub struct ExecMonitor {
    ebpf: Option<Bpf>,
    _monitor_task: Option<JoinHandle<()>>,
}

impl ExecMonitor {
    /// Create a new exec monitor instance
    pub fn new() -> Self {
        Self {
            ebpf: None,
            _monitor_task: None,
        }
    }

    /// Attach eBPF tracepoint to monitor exec syscalls
    ///
    /// # Returns
    /// Ok(()) if successful, Err if eBPF loading fails
    ///
    /// # Behavior
    /// - **Release mode**: Loads eBPF program from embedded bytecode (compiled at build time)
    /// - **Debug mode**: Loads from file for easier development, falls back to stub mode if not found
    /// - Attaches tracepoint to sched_process_exec
    /// - Starts background task to read and display events
    ///
    /// # Errors
    /// - If BPF program cannot be loaded
    /// - If program cannot be attached to tracepoint
    /// - If insufficient permissions (requires CAP_BPF or root)
    ///
    /// # Permission Requirements
    /// Loading eBPF programs requires special permissions:
    /// - **Linux 5.8+**: Requires CAP_BPF and CAP_PERFMON capabilities
    /// - **Older kernels**: Requires CAP_SYS_ADMIN capability
    /// - **Alternative**: Run as root (not recommended for production)
    ///
    /// Additionally, check kernel restrictions:
    /// - `kernel.unprivileged_bpf_disabled` should be 0 or 1 (not 2)
    /// - `kernel.perf_event_paranoid` should be <= 2
    pub async fn attach(&mut self) -> Result<()> {
        info!("eBPF exec monitor: attaching to sched_process_exec tracepoint");

        // Load eBPF program - use embedded bytes in release, file in debug
        let mut ebpf = {
            #[cfg(not(debug_assertions))]
            {
                info!("Loading embedded eBPF program for monitoring");
                match Bpf::load(super::EBPF_BYTES) {
                    Ok(ebpf) => ebpf,
                    Err(e) => {
                        return Err(JailError::Backend(format!(
                            "Failed to load embedded eBPF program for monitoring: {}",
                            e
                        )))
                    }
                }
            }

            #[cfg(debug_assertions)]
            {
                // In debug mode, try to load from file for easier development
                let ebpf_program_path = super::get_ebpf_program_path();
                if !std::path::Path::new(&ebpf_program_path).exists() {
                    debug_warn!("⚠️  eBPF program not found at: {}", ebpf_program_path);
                    debug_warn!("   Running in stub mode - exec monitoring will not be active");
                    debug_warn!("   To enable eBPF monitoring:");
                    debug_warn!("   1. Install Rust nightly: rustup install nightly");
                    debug_warn!("   2. Install bpf-linker: cargo install bpf-linker");
                    debug_warn!("   3. Build eBPF programs: cargo xtask build-ebpf --release");
                    return Ok(());
                }

                info!("Loading eBPF program from file (debug mode) for monitoring");
                match Bpf::load_file(&ebpf_program_path) {
                    Ok(ebpf) => ebpf,
                    Err(e) => {
                        return Err(JailError::Backend(format!(
                            "Failed to load eBPF program from file for monitoring: {}",
                            e
                        )))
                    }
                }
            }
        };

        // Load the tracepoint program
        debug!("Retrieving and loading trace_exec program");
        let program: &mut TracePoint = ebpf
            .program_mut("trace_exec")
            .ok_or_else(|| {
                JailError::Backend("trace_exec program not found in eBPF object".to_string())
            })?
            .try_into()
            .map_err(|e| {
                JailError::Backend(format!("Failed to convert to TracePoint program: {}", e))
            })?;

        debug!("Loading eBPF tracepoint program into kernel...");
        program.load().map_err(|e| {
            let errno = std::io::Error::last_os_error();

            // Check if this is a permission error
            if errno.kind() == std::io::ErrorKind::PermissionDenied {
                // Try to diagnose the permission issue
                let diagnostic = diagnose_bpf_permissions();
                JailError::Backend(format!(
                    "Failed to load tracepoint program into kernel: {} (errno: {:?})\n\n\
                    Permission Denied - eBPF programs require elevated privileges:\n\
                    {}\n\n\
                    Quick fix: Run with sudo:\n  sudo jail-ai <command> --monitor",
                    e, errno, diagnostic
                ))
            } else {
                JailError::Backend(format!(
                    "Failed to load tracepoint program into kernel: {} (errno: {:?})",
                    e, errno
                ))
            }
        })?;

        debug!("Attaching tracepoint to sched/sched_process_exec");
        program.attach("sched", "sched_process_exec").map_err(|e| {
            JailError::Backend(format!(
                "Failed to attach tracepoint: {} (errno: {:?})",
                e,
                std::io::Error::last_os_error()
            ))
        })?;

        info!("✓ Attached tracepoint to sched_process_exec");

        // Take ownership of perf event array map (removes it from ebpf object)
        // This allows us to move it into the async task while storing ebpf separately
        let perf_map = ebpf.take_map("EXEC_EVENTS").ok_or_else(|| {
            JailError::Backend("EXEC_EVENTS map not found in eBPF program".to_string())
        })?;

        let mut perf_array = AsyncPerfEventArray::try_from(perf_map).map_err(|e| {
            JailError::Backend(format!(
                "Failed to convert EXEC_EVENTS to AsyncPerfEventArray: {}",
                e
            ))
        })?;

        // Start background task to read events
        info!("Starting background task to read exec events");
        let monitor_task = tokio::spawn(async move {
            // Get online CPUs
            let cpus = match online_cpus() {
                Ok(cpus) => cpus,
                Err(e) => {
                    error!("Failed to get online CPUs: {}", e);
                    return;
                }
            };

            // Open perf buffers for each CPU
            let mut buffers = Vec::new();
            for cpu in cpus {
                match perf_array.open(cpu, Some(32)) {
                    Ok(buf) => buffers.push((cpu, buf)),
                    Err(e) => {
                        warn!("Failed to open perf buffer for CPU {}: {}", cpu, e);
                    }
                }
            }

            if buffers.is_empty() {
                error!("No perf buffers could be opened");
                return;
            }

            info!("✓ Exec monitoring active (reading from {} CPUs)", buffers.len());

            // Read events from all CPUs
            loop {
                for (cpu_id, buf) in &mut buffers {
                    let mut buffers = (0..10)
                        .map(|_| BytesMut::with_capacity(std::mem::size_of::<ExecEvent>()))
                        .collect::<Vec<_>>();

                    match buf.read_events(&mut buffers).await {
                        Ok(events) => {
                            for buf in buffers.iter().take(events.read) {
                                if buf.len() >= std::mem::size_of::<ExecEvent>() {
                                    // Safety: We know the buffer is large enough and properly aligned
                                    let event = unsafe {
                                        std::ptr::read_unaligned(buf.as_ptr() as *const ExecEvent)
                                    };
                                    print_exec_event(&event);
                                }
                            }
                        }
                        Err(e) => {
                            debug!("Error reading events from CPU {}: {}", cpu_id, e);
                        }
                    }
                }

                // Small sleep to avoid busy-waiting
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });

        // Store eBPF instance and monitor task
        self.ebpf = Some(ebpf);
        self._monitor_task = Some(monitor_task);

        info!("✓ eBPF exec monitoring active");
        Ok(())
    }

    /// Check if eBPF monitor is currently loaded
    #[allow(dead_code)]
    pub fn is_loaded(&self) -> bool {
        self.ebpf.is_some()
    }
}

impl Default for ExecMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ExecMonitor {
    fn drop(&mut self) {
        if self.ebpf.is_some() {
            debug!("eBPF exec monitor dropped, programs will be automatically detached");
        }
        if let Some(task) = self._monitor_task.take() {
            task.abort();
        }
    }
}

/// Diagnose BPF permission issues by checking kernel restrictions
fn diagnose_bpf_permissions() -> String {
    let mut diagnostics = Vec::new();

    // Check kernel.unprivileged_bpf_disabled
    if let Ok(content) = fs::read_to_string("/proc/sys/kernel/unprivileged_bpf_disabled") {
        let value = content.trim();
        match value {
            "2" => {
                diagnostics.push(
                    "• kernel.unprivileged_bpf_disabled = 2 (PERMANENTLY DISABLED)\n  \
                    This prevents BPF loading even with CAP_BPF capability.\n  \
                    To fix: Set to 1 in /etc/sysctl.d/99-bpf.conf and reboot:\n  \
                    echo 'kernel.unprivileged_bpf_disabled = 1' | sudo tee -a /etc/sysctl.d/99-bpf.conf"
                        .to_string(),
                );
            }
            "1" => {
                diagnostics.push(
                    "• kernel.unprivileged_bpf_disabled = 1 (requires CAP_BPF capability)"
                        .to_string(),
                );
            }
            _ => {}
        }
    }

    // Check kernel.perf_event_paranoid
    if let Ok(content) = fs::read_to_string("/proc/sys/kernel/perf_event_paranoid") {
        let value = content.trim();
        if let Ok(paranoid) = value.parse::<i32>() {
            if paranoid > 2 {
                diagnostics.push(format!(
                    "• kernel.perf_event_paranoid = {} (very restrictive)\n  \
                    Recommended: Set to 1 or lower:\n  \
                    echo 'kernel.perf_event_paranoid = 1' | sudo tee -a /etc/sysctl.d/99-bpf.conf",
                    paranoid
                ));
            }
        }
    }

    // Check capabilities (if running from a file)
    if let Ok(exe_path) = std::env::current_exe() {
        diagnostics.push(format!(
            "• Binary path: {}\n  \
            Check capabilities: sudo getcap {}",
            exe_path.display(),
            exe_path.display()
        ));
    }

    if diagnostics.is_empty() {
        "No specific diagnostic information available.\n\
        Ensure you have CAP_BPF and CAP_PERFMON capabilities, or run as root."
            .to_string()
    } else {
        diagnostics.join("\n\n")
    }
}

/// Print an exec event in a formatted way
fn print_exec_event(event: &ExecEvent) {
    // Convert timestamp to seconds.microseconds
    let secs = event.timestamp / 1_000_000_000;
    let usecs = (event.timestamp % 1_000_000_000) / 1_000;

    // Extract command name (null-terminated)
    let comm = String::from_utf8_lossy(&event.comm)
        .trim_end_matches('\0')
        .to_string();

    // Extract filename (null-terminated)
    let filename = String::from_utf8_lossy(&event.filename)
        .trim_end_matches('\0')
        .to_string();

    // Print in a format similar to bpftrace or strace
    info!(
        "[EXEC] {}.{:06} PID={} UID={} COMM={} FILE={}",
        secs, usecs, event.pid, event.uid, comm, filename
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitor_creation() {
        let monitor = ExecMonitor::new();
        assert!(!monitor.is_loaded());
    }
}
