//! Tauri IPC commands for Phase 2 security controls.

use crate::AppState;
use crate::commands::session_commands::{SessionRole, SessionState};
use tauri::{Manager, State};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct KioskPolicyOverride {
    pub security_mode: Option<String>,
    pub prevent_screenshots: Option<bool>,
    pub focus_watchdog: Option<bool>,
    pub controlled_paste: Option<bool>,
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
    session_state: State<'_, SessionState>,
    enabled: bool,
    policy: Option<KioskPolicyOverride>,
) -> serde_json::Value {
    log::info!("[Security] Kiosk mode requested: enabled={}", enabled);

    if enabled {
        let role = session_state.role.lock().unwrap().clone();
        if role != SessionRole::Student {
            log::warn!(
                "[Security] Ignoring kiosk enable because current role is not student: {:?}",
                role
            );
            return serde_json::json!({
                "success": false,
                "message": "Kiosk enable is allowed only for student role"
            });
        }
    }

    let cfg = state.config.lock().unwrap().clone();
    let security_mode = policy
        .as_ref()
        .and_then(|p| p.security_mode.clone())
        .unwrap_or_else(|| "monitor".to_string())
        .to_lowercase();
    let is_ultra = security_mode == "ultra";
    let is_lockdown = security_mode == "lockdown";
    let hard_lock_mode = is_ultra || is_lockdown;
    let effective_prevent_screenshots = policy
        .as_ref()
        .and_then(|p| p.prevent_screenshots)
        .unwrap_or(if hard_lock_mode { true } else { cfg.security.screenshot_prevention });
    let effective_focus_watchdog = policy
        .as_ref()
        .and_then(|p| p.focus_watchdog)
        .unwrap_or(if hard_lock_mode { true } else { cfg.security.focus_watchdog });
    let effective_controlled_paste = policy
        .as_ref()
        .and_then(|p| p.controlled_paste)
        .unwrap_or(!hard_lock_mode);

    let mut blocked_combinations = cfg.input_control.blocked_combinations.clone();
    if hard_lock_mode {
        blocked_combinations.extend([
            vec!["ctrl".to_string(), "c".to_string()],
            vec!["ctrl".to_string(), "v".to_string()],
            vec!["ctrl".to_string(), "x".to_string()],
            vec!["ctrl".to_string(), "insert".to_string()],
            vec!["shift".to_string(), "insert".to_string()],
        ]);
    }

    #[cfg(target_os = "windows")]
    {
        if enabled {
            crate::security::keyboard_hook::start_keyboard_hook(
                blocked_combinations,
            );

            if hard_lock_mode {
                // Wait briefly for hook installation to complete on the hook thread.
                for _ in 0..20 {
                    if crate::security::keyboard_hook::is_keyboard_hook_installed() {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(25));
                }

                if !crate::security::keyboard_hook::is_keyboard_hook_installed() {
                    log::error!("[Security] Hard-lock mode requested but keyboard hook is not installed");
                    return serde_json::json!({
                        "success": false,
                        "message": "Hard-lock protections failed to activate (keyboard hook unavailable)"
                    });
                }
            }

            crate::security::process_monitor::start_process_monitor(
                cfg.process_control.blacklist.clone(),
                cfg.process_control.monitor_interval_ms,
            );

            if !effective_controlled_paste {
                crate::security::clipboard_guard::start_clipboard_guard();
            } else {
                crate::security::clipboard_guard::stop_clipboard_guard();
                crate::security::clipboard_guard::clear_clipboard_once();
            }

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

            if let Some(win) = app.get_webview_window("main") {
                if hard_lock_mode {
                    let _ = win.set_decorations(false);
                    let _ = win.set_resizable(false);
                    let _ = win.set_always_on_top(true);
                    let _ = win.set_fullscreen(true);
                    let _ = win.set_minimizable(false);
                }
            }
        } else {
            crate::security::keyboard_hook::stop_keyboard_hook();
            crate::security::process_monitor::stop_process_monitor();
            crate::security::clipboard_guard::stop_clipboard_guard();
            crate::security::focus_watchdog::stop_focus_watchdog();
            crate::security::mouse_confinement::release_cursor();

            crate::security::screenshot_guard::disable_screenshot_prevention(None);

            if let Some(win) = app.get_webview_window("main") {
                let _ = win.set_fullscreen(false);
                let _ = win.set_always_on_top(false);
                let _ = win.set_resizable(true);
                let _ = win.set_decorations(true);
                let _ = win.set_minimizable(true);
            }
        }
    }

    serde_json::json!({
        "success": true,
        "message": if enabled { "Kiosk mode active" } else { "Kiosk mode disabled (exam mode may be unsafe)" }
    })
}
