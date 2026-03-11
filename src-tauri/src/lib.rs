mod commands;
mod config;
mod logger;
mod policy;
mod runtime;
mod security;
mod session;

use config::AppConfig;
use policy::engine::PolicyEngine;
use runtime::session::SessionManager;
use std::sync::Mutex;
use tauri::Manager;

use commands::session_commands::SessionState;
use session::db::SessionDb;
use std::sync::Arc;

// ─── Shared application state ───────────────────────────────────────────────

pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub policy_engine: Mutex<PolicyEngine>,
    pub session: Mutex<SessionManager>,
    pub running_process: Mutex<Option<RunningProcess>>,
}

/// Handle to an actively-running child process (code execution).
pub struct RunningProcess {
    pub pid: u32,
    pub stdin: Option<std::process::ChildStdin>,
}

// ─── Tauri entry point ──────────────────────────────────────────────────────

pub fn run() {
    let boot_config = config::AppConfig::load();
    logger::init(
        &boot_config.paths.logs,
        boot_config.logging.log_to_console,
        boot_config.logging.log_to_file,
    );
    log::info!("Starting Restricted IDE (Tauri)…");

    let config = boot_config;
    let policy_engine = PolicyEngine::new(&config);
    let session = SessionManager::new(&config);

    let kiosk_enabled = config.kiosk_mode.enabled;

    // Initialize session database
    let sessions_dir = config.paths.user_data.join("sessions");
    let _ = std::fs::create_dir_all(&sessions_dir);
    let db_path = sessions_dir.join("sessions.db");
    let session_db = Arc::new(
        SessionDb::open(&db_path).expect("Failed to open session database"),
    );
    let session_state = SessionState::new(session_db);

    let state = AppState {
        config: Mutex::new(config),
        policy_engine: Mutex::new(policy_engine),
        session: Mutex::new(session),
        running_process: Mutex::new(None),
    };

    tauri::Builder::default()
        .manage(state)
        .manage(session_state)
        .invoke_handler(tauri::generate_handler![
            // File-system commands
            commands::fs_commands::list_dir,
            commands::fs_commands::read_file,
            commands::fs_commands::write_file,
            commands::fs_commands::delete_file,
            commands::fs_commands::create_dir,
            commands::fs_commands::file_exists,
            commands::fs_commands::get_sandbox_path,
            commands::fs_commands::rename_file,
            // Admin commands
            commands::admin_commands::admin_login,
            commands::admin_commands::admin_logout,
            commands::admin_commands::admin_check_session,
            commands::admin_commands::admin_request_exit,
            commands::admin_commands::admin_get_logs,
            // Code execution
            commands::code_execution::run_code,
            commands::code_execution::stop_code,
            commands::code_execution::send_code_input,
            // System
            commands::system_commands::get_system_info,
            commands::system_commands::get_system_status,
            // Policy
            commands::policy_commands::validate_url,
            commands::policy_commands::validate_keyboard,
            commands::policy_commands::get_policy,
            commands::policy_commands::search_in_files,
            // Security
            commands::security_commands::check_vm,
            commands::security_commands::check_monitors,
            commands::security_commands::get_security_status,
            // Session
            commands::session_commands::create_session_cmd,
            commands::session_commands::start_session_cmd,
            commands::session_commands::end_session_cmd,
            commands::session_commands::delete_session_cmd,
            commands::session_commands::list_sessions_cmd,
            commands::session_commands::join_session_cmd,
            commands::session_commands::submit_code_cmd,
            commands::session_commands::heartbeat_cmd,
            commands::session_commands::get_session_status_cmd,
            commands::session_commands::get_session_participants_cmd,
            commands::session_commands::get_session_submissions_cmd,
            commands::session_commands::get_session_violations_cmd,
            commands::session_commands::broadcast_message_cmd,
            commands::session_commands::kick_participant_cmd,
            commands::session_commands::stop_lan_server_cmd,
            commands::session_commands::get_current_role_cmd,
            // Post-session
            commands::post_session_commands::judge_submissions_cmd,
            commands::post_session_commands::download_submissions_zip_cmd,
            commands::post_session_commands::export_results_csv_cmd,
            commands::post_session_commands::get_downloads_dir_cmd,
        ])
        .setup(move |app| {
            log::info!("Kiosk mode: {}", kiosk_enabled);

            // Maximize the window on startup so it fits the screen properly
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.maximize();
            }

            if kiosk_enabled {
                // Make the window fullscreen & always-on-top in kiosk mode
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_fullscreen(true);
                    let _ = window.set_always_on_top(true);
                    let _ = window.set_closable(false);
                    let _ = window.set_minimizable(false);
                }

                #[cfg(target_os = "windows")]
                {
                    let state = app.state::<AppState>();
                    let cfg = state.config.lock().unwrap();
                    let combos = cfg.input_control.blocked_combinations.clone();
                    let blacklist = cfg.process_control.blacklist.clone();
                    let interval = cfg.process_control.monitor_interval_ms;
                    let mouse_conf = cfg.input_control.mouse_confinement;
                    let security = cfg.security.clone();
                    drop(cfg);

                    security::keyboard_hook::start_keyboard_hook(combos);
                    security::process_monitor::start_process_monitor(blacklist, interval);
                    security::clipboard_guard::start_clipboard_guard();

                    // Confine mouse cursor after a short delay (window must be visible)
                    if mouse_conf {
                        std::thread::spawn(|| {
                            std::thread::sleep(std::time::Duration::from_millis(500));
                            security::mouse_confinement::confine_cursor_to_foreground();
                        });
                    }

                    // ── Phase 2 security controls ──

                    // VM detection
                    if security.vm_detection {
                        let vm_result = security::vm_detection::detect_vm();
                        if vm_result.is_vm {
                            log::error!(
                                "[Security] VM detected with {} indicator(s) — session may be blocked",
                                vm_result.indicators.len()
                            );
                        }
                    }

                    // Multi-monitor check
                    if security.multi_monitor_action != "ignore" {
                        let mon = security::monitor_detection::detect_monitors();
                        if mon.count > 1 {
                            log::warn!(
                                "[Security] {} monitors detected — action: {}",
                                mon.count,
                                security.multi_monitor_action
                            );
                        }
                    }

                    // Screenshot prevention (apply after short delay so window is ready)
                    if security.screenshot_prevention {
                        let win_handle = app.get_webview_window("main");
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_millis(800));
                            if let Some(w) = win_handle {
                                if let Ok(hwnd) = w.hwnd() {
                                    use windows::Win32::Foundation::HWND;
                                    let h = HWND(hwnd.0 as *mut _);
                                    security::screenshot_guard::enable_screenshot_prevention(Some(h));
                                }
                            }
                        });
                    }

                    // Focus-loss watchdog
                    if security.focus_watchdog {
                        let app_handle = app.handle().clone();
                        let poll = security.focus_poll_ms;
                        std::thread::spawn(move || {
                            // Wait for window to be fully ready
                            std::thread::sleep(std::time::Duration::from_millis(1000));
                            security::focus_watchdog::start_focus_watchdog(app_handle, poll);
                        });
                    }
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running application");
}
