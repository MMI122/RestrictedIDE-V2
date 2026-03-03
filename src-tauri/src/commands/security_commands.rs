//! Tauri IPC commands for Phase 2 security controls.

use crate::AppState;
use tauri::State;

/// Run VM detection and return the result.
#[tauri::command]
pub fn check_vm() -> crate::security::vm_detection::VmCheckResult {
    crate::security::vm_detection::detect_vm()
}

/// Enumerate monitors and return details.
#[tauri::command]
pub fn check_monitors() -> crate::security::monitor_detection::MonitorInfo {
    crate::security::monitor_detection::detect_monitors()
}

/// Get the current security status (combines all checks).
#[tauri::command]
pub fn get_security_status(
    state: State<'_, AppState>,
) -> SecurityStatus {
    let cfg = state.config.lock().unwrap();
    let security = &cfg.security;

    let vm_result = if security.vm_detection {
        Some(crate::security::vm_detection::detect_vm())
    } else {
        None
    };

    let monitor_result = if security.multi_monitor_action != "ignore" {
        Some(crate::security::monitor_detection::detect_monitors())
    } else {
        None
    };

    let is_blocked = {
        let vm_blocked = vm_result
            .as_ref()
            .map(|r| r.is_vm && security.vm_detection)
            .unwrap_or(false);
        let monitor_blocked = monitor_result
            .as_ref()
            .map(|r| r.count > 1 && security.multi_monitor_action == "deny")
            .unwrap_or(false);
        vm_blocked || monitor_blocked
    };

    SecurityStatus {
        vm: vm_result,
        monitors: monitor_result,
        screenshot_prevention: security.screenshot_prevention,
        focus_watchdog: security.focus_watchdog,
        is_blocked,
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SecurityStatus {
    pub vm: Option<crate::security::vm_detection::VmCheckResult>,
    pub monitors: Option<crate::security::monitor_detection::MonitorInfo>,
    pub screenshot_prevention: bool,
    pub focus_watchdog: bool,
    pub is_blocked: bool,
}
