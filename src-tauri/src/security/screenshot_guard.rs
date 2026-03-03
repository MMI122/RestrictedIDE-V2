//! Screenshot / screen-capture prevention.
//!
//! Uses `SetWindowDisplayAffinity(WDA_MONITOR)` to make the window
//! content appear black in screenshots and screen-sharing tools.

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, SetWindowDisplayAffinity, WINDOW_DISPLAY_AFFINITY,
};

/// WDA_MONITOR = 0x00000001 — content is only displayed on a monitor,
/// appears black in screen captures and remote desktop.
const WDA_MONITOR: WINDOW_DISPLAY_AFFINITY = WINDOW_DISPLAY_AFFINITY(0x00000001);
/// WDA_NONE = 0x00000000 — no restrictions.
const WDA_NONE: WINDOW_DISPLAY_AFFINITY = WINDOW_DISPLAY_AFFINITY(0x00000000);

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScreenshotGuardResult {
    pub enabled: bool,
    pub error: Option<String>,
}

/// Apply WDA_MONITOR to the given window handle to prevent screenshots.
/// If `hwnd` is null/0, uses the current foreground window.
pub fn enable_screenshot_prevention(hwnd: Option<HWND>) -> ScreenshotGuardResult {
    let target = hwnd.unwrap_or_else(|| unsafe { GetForegroundWindow() });

    if target.0.is_null() {
        let msg = "No valid window handle for screenshot prevention".to_string();
        log::warn!("[Security] {}", msg);
        return ScreenshotGuardResult {
            enabled: false,
            error: Some(msg),
        };
    }

    unsafe {
        match SetWindowDisplayAffinity(target, WDA_MONITOR) {
            Ok(()) => {
                log::info!(
                    "[Security] Screenshot prevention enabled (WDA_MONITOR) on HWND {:?}",
                    target.0
                );
                ScreenshotGuardResult {
                    enabled: true,
                    error: None,
                }
            }
            Err(e) => {
                let msg = format!("SetWindowDisplayAffinity failed: {}", e);
                log::warn!("[Security] {}", msg);
                ScreenshotGuardResult {
                    enabled: false,
                    error: Some(msg),
                }
            }
        }
    }
}

/// Remove screenshot prevention (e.g., for admin mode).
pub fn disable_screenshot_prevention(hwnd: Option<HWND>) -> ScreenshotGuardResult {
    let target = hwnd.unwrap_or_else(|| unsafe { GetForegroundWindow() });

    if target.0.is_null() {
        return ScreenshotGuardResult {
            enabled: false,
            error: Some("No valid window handle".into()),
        };
    }

    unsafe {
        match SetWindowDisplayAffinity(target, WDA_NONE) {
            Ok(()) => {
                log::info!("[Security] Screenshot prevention disabled on HWND {:?}", target.0);
                ScreenshotGuardResult {
                    enabled: false,
                    error: None,
                }
            }
            Err(e) => {
                let msg = format!("Failed to disable screenshot prevention: {}", e);
                log::warn!("[Security] {}", msg);
                ScreenshotGuardResult {
                    enabled: true,
                    error: Some(msg),
                }
            }
        }
    }
}
