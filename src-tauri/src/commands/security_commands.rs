//! Tauri IPC commands for Phase 2 security controls.

use crate::AppState;
use tauri::State;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct KioskPolicyOverride {
    pub prevent_screenshots: Option<bool>,
    pub focus_watchdog: Option<bool>,
}

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

/// Enable or disable kiosk mode lockdown.
#[tauri::command]
pub fn set_kiosk_mode(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
    policy: Option<KioskPolicyOverride>,
) -> serde_json::Value {
    log::info!("[Security] Kiosk mode requested: enabled={}", enabled);

    let cfg = state.config.lock().unwrap().clone();
    let effective_prevent_screenshots = policy
        .as_ref()
        .and_then(|p| p.prevent_screenshots)
        .unwrap_or(cfg.security.screenshot_prevention);
    let effective_focus_watchdog = policy
        .as_ref()
        .and_then(|p| p.focus_watchdog)
        .unwrap_or(cfg.security.focus_watchdog);

    #[cfg(target_os = "windows")]
    {
        if enabled {
            crate::security::keyboard_hook::start_keyboard_hook(
                cfg.input_control.blocked_combinations.clone(),
            );
            crate::security::process_monitor::start_process_monitor(
                cfg.process_control.blacklist.clone(),
                cfg.process_control.monitor_interval_ms,
            );
            crate::security::clipboard_guard::start_clipboard_guard();

            if cfg.input_control.mouse_confinement {
                crate::security::mouse_confinement::confine_cursor_to_foreground();
            }

            if effective_prevent_screenshots {
                crate::security::screenshot_guard::enable_screenshot_prevention(None);
            }

            if effective_focus_watchdog {
                crate::security::focus_watchdog::start_focus_watchdog(
                    app.clone(),
                    cfg.security.focus_poll_ms,
                );
            }
        } else {
            crate::security::keyboard_hook::stop_keyboard_hook();
            crate::security::process_monitor::stop_process_monitor();
            crate::security::clipboard_guard::stop_clipboard_guard();
            crate::security::focus_watchdog::stop_focus_watchdog();
            crate::security::mouse_confinement::release_cursor();

            crate::security::screenshot_guard::disable_screenshot_prevention(None);
        }
    }

    serde_json::json!({
        "success": true,
        "message": if enabled { "Kiosk mode active" } else { "Kiosk mode disabled (exam mode may be unsafe)" }
    })
}
