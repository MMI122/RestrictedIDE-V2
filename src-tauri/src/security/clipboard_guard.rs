//! Clipboard guard (Windows).
//!
//! Periodically clears the system clipboard to prevent data exfiltration.

use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard};

static CLIPBOARD_GUARD_RUNNING: AtomicBool = AtomicBool::new(false);

/// Clear the Windows clipboard.
fn clear_clipboard() {
    unsafe {
        if OpenClipboard(HWND::default()).is_ok() {
            let _ = EmptyClipboard();
            let _ = CloseClipboard();
        }
    }
}

/// Spawn a background thread that wipes the clipboard every few seconds.
pub fn start_clipboard_guard() {
    if CLIPBOARD_GUARD_RUNNING.swap(true, Ordering::SeqCst) {
        log::warn!("[Security] Clipboard guard already running");
        return;
    }

    std::thread::spawn(|| {
        log::info!("[Security] Clipboard guard started");

        loop {
            if !CLIPBOARD_GUARD_RUNNING.load(Ordering::SeqCst) {
                log::info!("[Security] Clipboard guard stopped");
                break;
            }

            std::thread::sleep(std::time::Duration::from_secs(3));
            clear_clipboard();
        }
    });
}

pub fn stop_clipboard_guard() {
    CLIPBOARD_GUARD_RUNNING.store(false, Ordering::SeqCst);
    log::info!("[Security] Clipboard guard stop requested");
}
