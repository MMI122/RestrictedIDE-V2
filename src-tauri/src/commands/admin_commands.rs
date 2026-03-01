use tauri::State;

use crate::AppState;

#[tauri::command]
pub fn admin_login(password: String, state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    let result = session.authenticate(&password);

    Ok(serde_json::json!({
        "success": result.success,
        "error": result.error,
        "attempts_remaining": result.attempts_remaining,
    }))
}

#[tauri::command]
pub fn admin_logout(state: State<'_, AppState>) -> Result<bool, String> {
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    session.logout();
    Ok(true)
}

#[tauri::command]
pub fn admin_check_session(state: State<'_, AppState>) -> Result<bool, String> {
    let session = state.session.lock().map_err(|e| e.to_string())?;
    Ok(session.is_authenticated())
}

#[tauri::command]
pub fn admin_request_exit(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let session = state.session.lock().map_err(|e| e.to_string())?;
    if !session.is_authenticated() {
        return Err("Admin authentication required".into());
    }
    drop(session);

    log::info!("[SECURITY] Admin exit requested");

    // Small delay so the response can reach the frontend
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(150));
        app.exit(0);
    });

    Ok(true)
}

#[tauri::command]
pub fn admin_get_logs(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let cfg = state.config.lock().map_err(|e| e.to_string())?;
    let log_dir = &cfg.paths.logs;

    let mut lines = Vec::new();

    if let Ok(entries) = std::fs::read_dir(log_dir) {
        let mut log_files: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map_or(false, |ext| ext == "log")
            })
            .collect();

        log_files.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

        if let Some(latest) = log_files.first() {
            if let Ok(content) = std::fs::read_to_string(latest.path()) {
                for line in content.lines().rev().take(500) {
                    lines.push(line.to_string());
                }
            }
        }
    }

    Ok(lines)
}
