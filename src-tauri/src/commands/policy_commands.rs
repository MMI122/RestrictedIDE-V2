use tauri::State;
use walkdir::WalkDir;
use once_cell::sync::Lazy;
use regex::Regex;
use std::time::Duration;
use url::Url;

use crate::AppState;
use crate::policy::engine::ValidationResult;

static RE_TITLE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?is)<title[^>]*>(.*?)</title>").unwrap());

#[derive(serde::Serialize)]
pub struct AllowedDocContent {
    pub url: String,
    pub title: String,
    pub html: String,
}

// ─── validate_url ───────────────────────────────────────────────────────────

#[tauri::command]
pub fn validate_url(url: String, state: State<'_, AppState>) -> Result<ValidationResult, String> {
    let engine = state.policy_engine.lock().map_err(|e| e.to_string())?;
    Ok(engine.validate_url(&url))
}

#[tauri::command]
pub fn fetch_allowed_doc_cmd(
    url: String,
    allowed_urls: Vec<String>,
    _state: State<'_, AppState>,
) -> Result<AllowedDocContent, String> {
    let normalized = url.trim();
    if normalized.is_empty() {
        return Err("URL is required".into());
    }

    let parsed = Url::parse(normalized).map_err(|_| "Invalid URL")?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("Only http/https URLs are allowed".into());
    }

    if !url_matches_allowlist(normalized, &allowed_urls) {
        return Err("URL not in whitelist".into());
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(normalized)
        .header(
            reqwest::header::USER_AGENT,
            "RestrictedIDE/1.0 (in-app-doc-viewer)",
        )
        .send()
        .map_err(|e| format!("Failed to fetch URL: {}", e))?;

    let final_url = response.url().to_string();
    if !url_matches_allowlist(&final_url, &allowed_urls) {
        return Err("Redirected URL not in whitelist".into());
    }

    let status = response.status();
    if !status.is_success() {
        return Err(format!("Failed to fetch URL: HTTP {}", status.as_u16()));
    }

    let html = response
        .text()
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    let title = extract_title(&html);

    Ok(AllowedDocContent {
        url: final_url,
        title,
        html,
    })
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

fn extract_title(html: &str) -> String {
    RE_TITLE
        .captures(html)
        .and_then(|c| c.get(1).map(|m| html_entity_decode(m.as_str().trim())))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Documentation".to_string())
}

fn html_entity_decode(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn url_matches_allowlist(url: &str, allowed_urls: &[String]) -> bool {
    let candidate = normalize_url(url);
    allowed_urls.iter().any(|entry| {
        let pattern = normalize_url(entry);
        if pattern.is_empty() {
            return false;
        }
        if let Some(prefix) = pattern.strip_suffix('*') {
            candidate.starts_with(prefix)
        } else {
            candidate == pattern
        }
    })
}

fn normalize_url(url: &str) -> String {
    url.trim().to_lowercase()
}
