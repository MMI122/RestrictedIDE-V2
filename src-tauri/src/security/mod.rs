#[cfg(target_os = "windows")]
pub mod clipboard_guard;
#[cfg(target_os = "windows")]
pub mod focus_watchdog;
#[cfg(target_os = "windows")]
pub mod keyboard_hook;
#[cfg(target_os = "windows")]
pub mod monitor_detection;
#[cfg(target_os = "windows")]
pub mod mouse_confinement;
#[cfg(target_os = "windows")]
pub mod process_monitor;
#[cfg(target_os = "windows")]
pub mod screenshot_guard;
#[cfg(target_os = "windows")]
pub mod vm_detection;

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
#[cfg(not(target_os = "windows"))]
pub mod vm_detection {
    #[derive(Debug, Clone, serde::Serialize)]
    pub struct VmCheckResult { pub is_vm: bool, pub indicators: Vec<String> }
    pub fn detect_vm() -> VmCheckResult {
        log::info!("[Security] VM detection not available on this platform");
        VmCheckResult { is_vm: false, indicators: vec![] }
    }
}
#[cfg(not(target_os = "windows"))]
pub mod monitor_detection {
    #[derive(Debug, Clone, serde::Serialize)]
    pub struct MonitorDetail { pub index: usize, pub left: i32, pub top: i32, pub right: i32, pub bottom: i32, pub is_primary: bool, pub device_name: String }
    #[derive(Debug, Clone, serde::Serialize)]
    pub struct MonitorInfo { pub count: usize, pub monitors: Vec<MonitorDetail> }
    pub fn detect_monitors() -> MonitorInfo {
        log::info!("[Security] Monitor detection not available on this platform");
        MonitorInfo { count: 1, monitors: vec![] }
    }
    pub fn is_multi_monitor() -> bool { false }
}
#[cfg(not(target_os = "windows"))]
pub mod screenshot_guard {
    #[derive(Debug, Clone, serde::Serialize)]
    pub struct ScreenshotGuardResult { pub enabled: bool, pub error: Option<String> }
    pub fn enable_screenshot_prevention(_hwnd: Option<()>) -> ScreenshotGuardResult {
        log::info!("[Security] Screenshot prevention not available on this platform");
        ScreenshotGuardResult { enabled: false, error: Some("Not supported".into()) }
    }
}
#[cfg(not(target_os = "windows"))]
pub mod focus_watchdog {
    pub fn start_focus_watchdog(_app: tauri::AppHandle, _poll_ms: u64) {
        log::info!("[Security] Focus watchdog not available on this platform");
    }
    pub fn stop_focus_watchdog() {}
}
