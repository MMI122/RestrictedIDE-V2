//! Clipboard guard (Windows).
//!
//! Periodically clears the system clipboard to prevent data exfiltration.

use windows::Win32::Foundation::HWND;
use windows::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard};

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
    std::thread::spawn(|| {
        log::info!("[Security] Clipboard guard started");

        loop {
            std::thread::sleep(std::time::Duration::from_secs(3));
            clear_clipboard();
        }
    });
}
