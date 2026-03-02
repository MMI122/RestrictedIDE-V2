use std::fs;
use std::path::PathBuf;

/// Initialize the application logger.
/// Writes structured log lines to both the console (in dev) and a daily log file.
pub fn init(log_dir: &PathBuf, log_to_console: bool, log_to_file: bool) {
    let level = if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    // Ensure log directory exists
    let _ = fs::create_dir_all(log_dir);

    let log_file_path = log_dir.join(format!(
        "audit-{}.log",
        chrono::Local::now().format("%Y-%m-%d")
    ));

    let mut dispatch = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{{\"ts\":\"{}\",\"level\":\"{}\",\"target\":\"{}\",\"msg\":\"{}\"}}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f"),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(level)
        // Suppress noisy crates
        .level_for("tao", log::LevelFilter::Warn)
        .level_for("wry", log::LevelFilter::Warn)
        .level_for("tauri", log::LevelFilter::Warn);

    if log_to_console {
        dispatch = dispatch.chain(std::io::stdout());
    }

    if log_to_file {
        match fern::log_file(&log_file_path) {
            Ok(file) => {
                dispatch = dispatch.chain(file);
            }
            Err(e) => {
                eprintln!("[Logger] Cannot open log file {:?}: {}", log_file_path, e);
                // Fall back to stdout-only
                if !log_to_console {
                    dispatch = dispatch.chain(std::io::stdout());
                }
            }
        }
    }

    dispatch.apply().unwrap_or_else(|e| {
        eprintln!("[Logger] Failed to init logger: {}", e);
    });
}
