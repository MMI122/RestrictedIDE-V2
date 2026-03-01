use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// ─── Top-level app config ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub app_name: String,
    pub version: String,
    pub is_development: bool,
    pub paths: PathsConfig,
    pub kiosk_mode: KioskConfig,
    pub input_control: InputControlConfig,
    pub process_control: ProcessControlConfig,
    pub fs_sandbox: FsSandboxConfig,
    pub admin: AdminConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    pub root: PathBuf,
    pub user_data: PathBuf,
    pub logs: PathBuf,
    pub policies: PathBuf,
    pub config_dir: PathBuf,
    pub sandbox: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KioskConfig {
    pub enabled: bool,
    pub admin_can_close: bool,
    pub admin_access_combo: Vec<String>,
    pub admin_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputControlConfig {
    pub blocked_combinations: Vec<Vec<String>>,
    pub mouse_confinement: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessControlConfig {
    pub monitor_interval_ms: u64,
    pub whitelist: Vec<String>,
    pub blacklist: Vec<String>,
    pub kill_unauthorized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsSandboxConfig {
    pub sandbox_root: PathBuf,
    pub allowed_extensions: Vec<String>,
    pub max_file_size: u64,
    pub denied_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfig {
    pub session_timeout_ms: u64,
    pub max_login_attempts: u32,
    pub lockout_duration_ms: u64,
    pub secret_key_combo: Vec<String>,
    pub password_min_length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub max_file_size: u64,
    pub max_files: u32,
    pub log_to_console: bool,
    pub log_to_file: bool,
}

// ─── Builder ────────────────────────────────────────────────────────────────

impl AppConfig {
    /// Build the config from environment, creating directories as needed.
    pub fn load() -> Self {
        let is_dev =
            cfg!(debug_assertions) || std::env::var("TAURI_ENV").unwrap_or_default() == "development";
        let force_kiosk = std::env::var("FORCE_KIOSK").unwrap_or_default() == "true";

        let app_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // User-data location (platform-specific)
        let user_data = if is_dev {
            app_root.join("dev-data")
        } else if cfg!(target_os = "windows") {
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("C:\\ProgramData"))
                .join("RestrictedIDE")
        } else if cfg!(target_os = "macos") {
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("RestrictedIDE")
        } else {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join(".restricted-ide")
        };

        let sandbox = user_data.join("sandbox");
        let logs = user_data.join("logs");
        let policies = user_data.join("policies");
        let config_dir = user_data.join("config");

        // Create essential dirs
        for dir in [&user_data, &sandbox, &logs, &policies, &config_dir] {
            let _ = fs::create_dir_all(dir);
        }

        // --- platform-specific blocked key combos ---
        let blocked_combinations = if cfg!(target_os = "windows") {
            vec![
                vec!["alt".into(), "tab".into()],
                vec!["alt".into(), "f4".into()],
                vec!["alt".into(), "escape".into()],
                vec!["ctrl".into(), "escape".into()],
                vec!["ctrl".into(), "shift".into(), "escape".into()],
                vec!["win".into()],
                vec!["win".into(), "d".into()],
                vec!["win".into(), "e".into()],
                vec!["win".into(), "r".into()],
                vec!["win".into(), "l".into()],
                vec!["ctrl".into(), "alt".into(), "delete".into()],
                vec!["f11".into()],
                vec!["f12".into()],
                vec!["ctrl".into(), "shift".into(), "i".into()],
            ]
        } else {
            vec![
                vec!["cmd".into(), "tab".into()],
                vec!["cmd".into(), "q".into()],
                vec!["cmd".into(), "w".into()],
                vec!["cmd".into(), "space".into()],
                vec!["cmd".into(), "option".into(), "escape".into()],
                vec!["f11".into()],
                vec!["f12".into()],
                vec!["cmd".into(), "option".into(), "i".into()],
            ]
        };

        // --- process lists ---
        let process_whitelist: Vec<String> = if cfg!(target_os = "windows") {
            vec![
                "csrss.exe", "smss.exe", "services.exe", "lsass.exe", "svchost.exe",
                "conhost.exe", "dwm.exe", "winlogon.exe", "wininit.exe",
                "restricted-ide.exe", "node.exe",
            ]
        } else {
            vec![
                "kernel_task", "launchd", "WindowServer", "restricted-ide", "node",
            ]
        }
        .into_iter()
        .map(String::from)
        .collect();

        let process_blacklist: Vec<String> = if cfg!(target_os = "windows") {
            vec![
                "cmd.exe", "powershell.exe", "pwsh.exe", "taskmgr.exe", "regedit.exe",
                "mmc.exe", "control.exe", "notepad.exe", "chrome.exe", "firefox.exe",
                "msedge.exe",
            ]
        } else {
            vec![
                "terminal", "iterm2", "activity monitor", "safari", "google chrome",
                "firefox",
            ]
        }
        .into_iter()
        .map(String::from)
        .collect();

        // --- denied filesystem paths ---
        let denied_paths: Vec<PathBuf> = if cfg!(target_os = "windows") {
            vec![
                PathBuf::from("C:\\Windows"),
                PathBuf::from("C:\\Program Files"),
                PathBuf::from("C:\\Program Files (x86)"),
            ]
        } else {
            vec![
                PathBuf::from("/System"),
                PathBuf::from("/Library"),
                PathBuf::from("/Applications"),
                PathBuf::from("/bin"),
                PathBuf::from("/usr/bin"),
            ]
        };

        AppConfig {
            app_name: "Restricted IDE".into(),
            version: "0.1.0".into(),
            is_development: is_dev,
            paths: PathsConfig {
                root: app_root,
                user_data,
                logs,
                policies,
                config_dir,
                sandbox: sandbox.clone(),
            },
            kiosk_mode: KioskConfig {
                enabled: force_kiosk || !is_dev,
                admin_can_close: true,
                admin_access_combo: vec![
                    "ctrl".into(),
                    "shift".into(),
                    "alt".into(),
                    "a".into(),
                ],
                admin_timeout_ms: 300_000,
            },
            input_control: InputControlConfig {
                blocked_combinations,
                mouse_confinement: true,
            },
            process_control: ProcessControlConfig {
                monitor_interval_ms: 2000,
                whitelist: process_whitelist,
                blacklist: process_blacklist,
                kill_unauthorized: true,
            },
            fs_sandbox: FsSandboxConfig {
                sandbox_root: sandbox,
                allowed_extensions: vec![
                    ".txt", ".md", ".json", ".js", ".ts", ".jsx", ".tsx", ".py", ".java",
                    ".c", ".cpp", ".h", ".hpp", ".html", ".css", ".scss", ".xml", ".yaml",
                    ".yml",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
                max_file_size: 10 * 1024 * 1024, // 10 MB
                denied_paths,
            },
            admin: AdminConfig {
                session_timeout_ms: 300_000,       // 5 min
                max_login_attempts: 3,
                lockout_duration_ms: 300_000,      // 5 min
                secret_key_combo: vec![
                    "ctrl".into(),
                    "shift".into(),
                    "alt".into(),
                    "a".into(),
                ],
                password_min_length: 8,
            },
            logging: LoggingConfig {
                level: if is_dev { "debug".into() } else { "info".into() },
                max_file_size: 10 * 1024 * 1024,
                max_files: 10,
                log_to_console: is_dev,
                log_to_file: true,
            },
        }
    }
}
