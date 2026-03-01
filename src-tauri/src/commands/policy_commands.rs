use tauri::State;
use walkdir::WalkDir;

use crate::AppState;
use crate::policy::engine::ValidationResult;

// ─── validate_url ───────────────────────────────────────────────────────────

#[tauri::command]
pub fn validate_url(url: String, state: State<'_, AppState>) -> Result<ValidationResult, String> {
    let engine = state.policy_engine.lock().map_err(|e| e.to_string())?;
    Ok(engine.validate_url(&url))
}

// ─── validate_keyboard ─────────────────────────────────────────────────────

#[tauri::command]
pub fn validate_keyboard(
    keys: Vec<String>,
    state: State<'_, AppState>,
) -> Result<ValidationResult, String> {
    let engine = state.policy_engine.lock().map_err(|e| e.to_string())?;
    Ok(engine.validate_keyboard(&keys))
}

// ─── get_policy ─────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_policy(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let cfg = state.config.lock().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "kiosk_mode": cfg.kiosk_mode.enabled,
        "sandbox_root": cfg.fs_sandbox.sandbox_root.to_string_lossy(),
        "allowed_extensions": cfg.fs_sandbox.allowed_extensions,
        "blocked_combinations": cfg.input_control.blocked_combinations,
        "process_blacklist": cfg.process_control.blacklist,
    }))
}

// ─── search_in_files ────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct SearchMatch {
    pub file: String,
    pub line: u32,
    pub text: String,
}

#[tauri::command]
pub fn search_in_files(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<SearchMatch>, String> {
    if query.is_empty() {
        return Ok(vec![]);
    }

    let cfg = state.config.lock().map_err(|e| e.to_string())?;
    let sandbox = &cfg.fs_sandbox.sandbox_root;

    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for entry in WalkDir::new(sandbox)
        .max_depth(10)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();

        // Only search text files
        let ext = path
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
            .unwrap_or_default();

        if !is_text_extension(&ext) {
            continue;
        }

        if let Ok(content) = std::fs::read_to_string(path) {
            for (i, line) in content.lines().enumerate() {
                if line.to_lowercase().contains(&query_lower) {
                    results.push(SearchMatch {
                        file: path.to_string_lossy().into(),
                        line: (i + 1) as u32,
                        text: line.to_string(),
                    });
                }
                if results.len() >= 200 {
                    return Ok(results);
                }
            }
        }
    }

    Ok(results)
}

fn is_text_extension(ext: &str) -> bool {
    matches!(
        ext,
        ".txt" | ".md" | ".json" | ".js" | ".ts" | ".jsx" | ".tsx"
            | ".py" | ".java" | ".c" | ".cpp" | ".h" | ".hpp"
            | ".html" | ".css" | ".scss" | ".xml" | ".yaml" | ".yml"
    )
}
