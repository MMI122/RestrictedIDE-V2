mod commands;
mod config;
mod logger;
mod policy;
mod runtime;
mod security;

use config::AppConfig;
use policy::engine::PolicyEngine;
use runtime::session::SessionManager;
use std::sync::Mutex;
use tauri::Manager;

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
    logger::init();
    log::info!("Starting Restricted IDE (Tauri)…");

    let config = AppConfig::load();
    let policy_engine = PolicyEngine::new(&config);
    let session = SessionManager::new(&config);

    let kiosk_enabled = config.kiosk_mode.enabled;

    let state = AppState {
        config: Mutex::new(config),
        policy_engine: Mutex::new(policy_engine),
        session: Mutex::new(session),
        running_process: Mutex::new(None),
    };

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            // File-system commands
            commands::fs_commands::list_dir,
            commands::fs_commands::read_file,
            commands::fs_commands::write_file,
            commands::fs_commands::delete_file,
            commands::fs_commands::create_dir,
            commands::fs_commands::file_exists,
            commands::fs_commands::get_sandbox_path,
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
                    drop(cfg);

                    security::keyboard_hook::start_keyboard_hook(combos);
                    security::process_monitor::start_process_monitor(blacklist, interval);
                    security::clipboard_guard::start_clipboard_guard();
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running application");
}
