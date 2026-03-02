#[cfg(target_os = "windows")]
pub mod clipboard_guard;
#[cfg(target_os = "windows")]
pub mod keyboard_hook;
#[cfg(target_os = "windows")]
pub mod mouse_confinement;
#[cfg(target_os = "windows")]
pub mod process_monitor;

// Re-export start functions only on Windows; no-ops otherwise.
#[cfg(not(target_os = "windows"))]
pub mod keyboard_hook {
    pub fn start_keyboard_hook(_combos: Vec<Vec<String>>) {
        log::info!("[Security] Keyboard hook not available on this platform");
    }
}
#[cfg(not(target_os = "windows"))]
pub mod process_monitor {
    pub fn start_process_monitor(_blacklist: Vec<String>, _interval: u64) {
        log::info!("[Security] Process monitor not available on this platform");
    }
}
#[cfg(not(target_os = "windows"))]
pub mod clipboard_guard {
    pub fn start_clipboard_guard() {
        log::info!("[Security] Clipboard guard not available on this platform");
    }
}
