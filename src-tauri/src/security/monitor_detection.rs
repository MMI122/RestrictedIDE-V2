//! Multi-monitor detection and enforcement.
//!
//! Detects how many monitors are connected and can optionally
//! blackout secondary monitors or just report.

use windows::Win32::Foundation::{BOOL, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW, MONITORINFO,
};

#[derive(Debug, Clone, serde::Serialize)]
pub struct MonitorInfo {
    pub count: usize,
    pub monitors: Vec<MonitorDetail>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MonitorDetail {
    pub index: usize,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub is_primary: bool,
    pub device_name: String,
}

/// Enumerate all connected monitors and return details.
pub fn detect_monitors() -> MonitorInfo {
    let mut monitors: Vec<MonitorDetail> = Vec::new();

    unsafe {
        let _ = EnumDisplayMonitors(
            HDC::default(),
            None,
            Some(monitor_enum_proc),
            LPARAM(&mut monitors as *mut Vec<MonitorDetail> as isize),
        );
    }

    let count = monitors.len();

    if count > 1 {
        log::warn!(
            "[Security] Multiple monitors detected: {} monitor(s)",
            count
        );
    } else {
        log::info!("[Security] Monitor check passed — {} monitor(s)", count);
    }

    MonitorInfo { count, monitors }
}

/// Callback for EnumDisplayMonitors.
unsafe extern "system" fn monitor_enum_proc(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _lprect: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let monitors = &mut *(lparam.0 as *mut Vec<MonitorDetail>);
    let index = monitors.len();

    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

    if GetMonitorInfoW(hmonitor, &mut info as *mut MONITORINFOEXW as *mut MONITORINFO).as_bool() {
        let rc = info.monitorInfo.rcMonitor;
        let is_primary = (info.monitorInfo.dwFlags & 1) != 0; // MONITORINFOF_PRIMARY = 1
        let device_name = String::from_utf16_lossy(
            &info
                .szDevice
                .iter()
                .take_while(|&&c| c != 0)
                .copied()
                .collect::<Vec<u16>>(),
        );

        monitors.push(MonitorDetail {
            index,
            left: rc.left,
            top: rc.top,
            right: rc.right,
            bottom: rc.bottom,
            is_primary,
            device_name,
        });
    }

    BOOL(1) // continue enumeration
}

/// Check if multi-monitor policy is violated.
/// Returns true if there is more than one monitor.
pub fn is_multi_monitor() -> bool {
    let info = detect_monitors();
    info.count > 1
}
