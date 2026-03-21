use windows::Win32::Foundation::RECT;
use windows::Win32::UI::WindowsAndMessaging::{ClipCursor, GetForegroundWindow, GetWindowRect};

/// Confine the mouse cursor to the foreground window's rectangle.
/// Called once on kiosk activation and should be re-called on resize events.
pub fn confine_cursor_to_foreground() {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            log::warn!("[Mouse] No foreground window found for cursor confinement");
            return;
        }
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            match ClipCursor(Some(&rect)) {
                Ok(()) => {
                    log::info!(
                        "[Security] Mouse confined to rect ({},{}) -> ({},{})",
                        rect.left,
                        rect.top,
                        rect.right,
                        rect.bottom
                    );
                }
                Err(e) => {
                    log::warn!("[Mouse] ClipCursor failed: {}", e);
                }
            }
        } else {
            log::warn!("[Mouse] GetWindowRect failed");
        }
    }
}

/// Release cursor confinement (e.g., on admin exit).
#[allow(dead_code)] // used by future post-session unlock sequence
pub fn release_cursor() {
    unsafe {
        let _ = ClipCursor(None);
        log::info!("[Security] Mouse confinement released");
    }
}
