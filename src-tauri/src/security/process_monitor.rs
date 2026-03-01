//! Background process monitor (Windows).
//!
//! Periodically scans running processes and kills any that appear on the
//! blacklist.

use std::collections::HashSet;
use sysinfo::{ProcessesToUpdate, System};

/// Spawn a background thread that polls processes at the configured interval.
pub fn start_process_monitor(blacklist: Vec<String>, interval_ms: u64) {
    if blacklist.is_empty() {
        log::info!("[Security] Process blacklist empty – monitor skipped");
        return;
    }

    let blocked: HashSet<String> = blacklist.into_iter().map(|s| s.to_lowercase()).collect();

    std::thread::spawn(move || {
        log::info!("[Security] Process monitor started (interval {}ms)", interval_ms);

        let mut sys = System::new();

        loop {
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
