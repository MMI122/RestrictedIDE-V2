//! Background process monitor (Windows).
//!
//! Periodically scans running processes and kills any that appear on the
//! blacklist.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use sysinfo::{ProcessesToUpdate, System};

static PROCESS_MONITOR_RUNNING: AtomicBool = AtomicBool::new(false);

/// Spawn a background thread that polls processes at the configured interval.
pub fn start_process_monitor(blacklist: Vec<String>, interval_ms: u64) {
    if PROCESS_MONITOR_RUNNING.swap(true, Ordering::SeqCst) {
        log::warn!("[Security] Process monitor already running");
        return;
    }

    if blacklist.is_empty() {
        PROCESS_MONITOR_RUNNING.store(false, Ordering::SeqCst);
        log::info!("[Security] Process blacklist empty – monitor skipped");
        return;
    }

    let blocked: HashSet<String> = blacklist.into_iter().map(|s| s.to_lowercase()).collect();

    std::thread::spawn(move || {
        log::info!("[Security] Process monitor started (interval {}ms)", interval_ms);

        let mut sys = System::new();

        loop {
            if !PROCESS_MONITOR_RUNNING.load(Ordering::SeqCst) {
                log::info!("[Security] Process monitor stopped");
                break;
            }

            std::thread::sleep(std::time::Duration::from_millis(interval_ms));

            sys.refresh_processes(ProcessesToUpdate::All, true);

            for (_pid, process) in sys.processes() {
                let name = process.name().to_string_lossy().to_lowercase();

                if blocked.contains(&name) {
                    log::warn!("[Security] Killing blacklisted process: {} (PID {})", name, _pid);
                    process.kill();
                }
            }
        }
    });
}

pub fn stop_process_monitor() {
    PROCESS_MONITOR_RUNNING.store(false, Ordering::SeqCst);
    log::info!("[Security] Process monitor stop requested");
}
