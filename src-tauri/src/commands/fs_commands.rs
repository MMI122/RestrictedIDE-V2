use serde::Serialize;
use std::fs;
use std::path::Path;
use tauri::State;

use crate::AppState;

// ─── Types ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub is_file: bool,
}

// ─── Commands ───────────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_dir(dir_path: String, state: State<'_, AppState>) -> Result<Vec<FileEntry>, String> {
    // Policy check
    {
        let engine = state.policy_engine.lock().map_err(|e| e.to_string())?;
        let r = engine.validate_file_access(&dir_path, "read");
        if !r.allowed {
            return Err(r.reason.unwrap_or_else(|| "Access denied".into()));
        }
    }

    let entries = fs::read_dir(&dir_path).map_err(|e| format!("Cannot read dir: {}", e))?;

    let mut files: Vec<FileEntry> = entries
        .filter_map(|e| {
            let e = e.ok()?;
            let meta = e.metadata().ok()?;
            Some(FileEntry {
                name: e.file_name().to_string_lossy().into(),
                path: e.path().to_string_lossy().into(),
                is_directory: meta.is_dir(),
                is_file: meta.is_file(),
            })
        })
        .collect();

    // Folders first, then alphabetical
    files.sort_by(|a, b| {
        b.is_directory
            .cmp(&a.is_directory)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(files)
}

#[tauri::command]
pub fn read_file(file_path: String, state: State<'_, AppState>) -> Result<String, String> {
    {
        let engine = state.policy_engine.lock().map_err(|e| e.to_string())?;
        let r = engine.validate_file_access(&file_path, "read");
        if !r.allowed {
            return Err(r.reason.unwrap_or_else(|| "Access denied".into()));
        }
    }
    fs::read_to_string(&file_path).map_err(|e| format!("Cannot read file: {}", e))
}

#[tauri::command]
pub fn write_file(
    file_path: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let engine = state.policy_engine.lock().map_err(|e| e.to_string())?;
        let r = engine.validate_file_access(&file_path, "write");
        if !r.allowed {
            return Err(r.reason.unwrap_or_else(|| "Access denied".into()));
        }
    }

    if let Some(parent) = Path::new(&file_path).parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&file_path, &content).map_err(|e| format!("Cannot write file: {}", e))?;
    log::info!("[AUDIT] FILE_WRITE: {}", file_path);
    Ok(())
}

#[tauri::command]
pub fn delete_file(file_path: String, state: State<'_, AppState>) -> Result<(), String> {
    {
        let engine = state.policy_engine.lock().map_err(|e| e.to_string())?;
        let r = engine.validate_file_access(&file_path, "delete");
        if !r.allowed {
            return Err(r.reason.unwrap_or_else(|| "Access denied".into()));
        }
    }

    let path = Path::new(&file_path);
    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|e| e.to_string())?;
    } else {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    log::info!("[AUDIT] FILE_DELETE: {}", file_path);
    Ok(())
}

#[tauri::command]
pub fn create_dir(dir_path: String, state: State<'_, AppState>) -> Result<(), String> {
    {
        let engine = state.policy_engine.lock().map_err(|e| e.to_string())?;
        let r = engine.validate_file_access(&dir_path, "write");
        if !r.allowed {
            return Err(r.reason.unwrap_or_else(|| "Access denied".into()));
        }
    }
    fs::create_dir_all(&dir_path).map_err(|e| e.to_string())?;
    log::info!("[AUDIT] DIR_CREATE: {}", dir_path);
    Ok(())
}

#[tauri::command]
pub fn file_exists(file_path: String) -> Result<bool, String> {
    Ok(Path::new(&file_path).exists())
}

#[tauri::command]
pub fn get_sandbox_path(state: State<'_, AppState>) -> Result<String, String> {
    let cfg = state.config.lock().map_err(|e| e.to_string())?;
    Ok(cfg.fs_sandbox.sandbox_root.to_string_lossy().into())
}
