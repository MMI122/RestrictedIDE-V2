//! Focus-loss watchdog.
//!
//! Periodically checks whether the IDE window is the foreground window.
//! When focus is lost, emits a Tauri event so the frontend can react
//! and logs a security violation.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

static WATCHDOG_RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, serde::Serialize)]
pub struct FocusEvent {
    pub has_focus: bool,
    pub timestamp: String,
    pub consecutive_losses: u32,
}

/// Start the focus-loss watchdog loop on a background thread.
/// Emits `security://focus-change` events to the frontend.
/// `poll_ms` controls how often to check (default 500ms).
/// `our_pid` is the PID of the IDE process to compare against.
pub fn start_focus_watchdog(app: AppHandle, poll_ms: u64) {
    if WATCHDOG_RUNNING.swap(true, Ordering::SeqCst) {
        log::warn!("[Security] Focus watchdog already running");
        return;
    }

    let interval = Duration::from_millis(poll_ms);

    std::thread::spawn(move || {
        log::info!(
            "[Security] Focus watchdog started (poll interval: {}ms)",
            poll_ms
        );

        let mut had_focus = true;
        let mut consecutive_losses: u32 = 0;

        // Get our window's HWND for comparison
        let our_hwnd = {
            if let Some(window) = app.get_webview_window("main") {
                match window.hwnd() {
                    Ok(hwnd) => Some(hwnd.0 as isize),
                    Err(_) => None,
                }
            } else {
                None
            }
        };

        loop {
            if !WATCHDOG_RUNNING.load(Ordering::SeqCst) {
                log::info!("[Security] Focus watchdog stopped");
                break;
            }

            std::thread::sleep(interval);

            let has_focus = unsafe {
                let fg = GetForegroundWindow();
                if fg.0.is_null() {
                    false
                } else {
                    match our_hwnd {
                        Some(our) => fg.0 as isize == our,
                        None => true, // Can't compare, assume OK
                    }
                }
            };

            if has_focus != had_focus {
                if !has_focus {
                    consecutive_losses += 1;
                    log::warn!(
                        "[Security] FOCUS LOST — violation #{} at {}",
                        consecutive_losses,
                        chrono::Local::now().format("%H:%M:%S")
                    );
                } else {
                    log::info!(
                        "[Security] Focus regained at {}",
                        chrono::Local::now().format("%H:%M:%S")
                    );
                }

                let event = FocusEvent {
                    has_focus,
                    timestamp: chrono::Local::now().to_rfc3339(),
                    consecutive_losses,
                };

                let _ = app.emit("security://focus-change", &event);
                had_focus = has_focus;
            }
        }
    });
}

/// Stop the focus watchdog loop.
#[allow(dead_code)] // lifecycle hook reserved for explicit kiosk teardown flow
pub fn stop_focus_watchdog() {
    WATCHDOG_RUNNING.store(false, Ordering::SeqCst);
    log::info!("[Security] Focus watchdog stop requested");
}
