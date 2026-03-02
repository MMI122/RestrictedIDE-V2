use serde::Serialize;

use super::file_access_rule::FileAccessRule;
use super::keyboard_rule::KeyboardRule;
use super::process_rule::ProcessRule;
use super::time_rule::TimeRule;
use super::url_rule::UrlRule;
use crate::config::AppConfig;

// ─── Shared validation result ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ValidationResult {
    pub allowed: bool,
    pub reason: Option<String>,
}

// ─── Policy Engine ──────────────────────────────────────────────────────────

pub struct PolicyEngine {
    pub url_rule: UrlRule,
    pub keyboard_rule: KeyboardRule,
    pub process_rule: ProcessRule,
    pub file_access_rule: FileAccessRule,
    pub time_rule: TimeRule,
}

impl PolicyEngine {
    pub fn new(config: &AppConfig) -> Self {
        // Default whitelisted URL patterns
        let url_patterns = vec![
            "https://developer.mozilla.org/*".into(),
            "https://docs.python.org/*".into(),
            "https://cplusplus.com/*".into(),
            "https://www.w3schools.com/*".into(),
            "https://docs.oracle.com/javase/*".into(),
        ];

        let blocked_combos: Vec<(Vec<String>, String)> = config
            .input_control
            .blocked_combinations
            .iter()
            .map(|keys| (keys.clone(), "Policy restriction".into()))
            .collect();

        let sandbox = config.fs_sandbox.sandbox_root.to_string_lossy().to_string();
        let allowed_ext = config.fs_sandbox.allowed_extensions.clone();
        let denied: Vec<String> = config
            .fs_sandbox
            .denied_paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        log::info!("PolicyEngine initialised – sandbox: {}", sandbox);

        Self {
            url_rule: UrlRule::new("whitelist", url_patterns),
            keyboard_rule: KeyboardRule::new("blacklist", blocked_combos),
            process_rule: ProcessRule::new(
                "whitelist",
                config.process_control.whitelist.clone(),
                config.process_control.blacklist.clone(),
            ),
            file_access_rule: FileAccessRule::new(
                "sandbox",
                &sandbox,
                allowed_ext,
                config.fs_sandbox.max_file_size,
                denied,
            ),
            time_rule: TimeRule::new(false, None),
        }
    }

    // ---- validators ----

    pub fn validate_url(&self, url: &str) -> ValidationResult {
        let r = self.url_rule.validate(url);
        if !r.allowed {
            log::warn!("[Policy] URL blocked: {}", url);
        }
        r
    }

    pub fn validate_keyboard(&self, keys: &[String]) -> ValidationResult {
        let r = self.keyboard_rule.validate(keys);
        if !r.allowed {
            log::warn!("[Policy] Keyboard combo blocked: {:?}", keys);
        }
        r
    }

    pub fn validate_process(&self, name: &str) -> ValidationResult {
        self.process_rule.validate(name)
    }

    pub fn validate_file_size(&self, size: u64) -> ValidationResult {
        let r = self.file_access_rule.validate_file_size(size);
        if !r.allowed {
            log::warn!("[Policy] File size rejected: {} bytes", size);
        }
        r
    }

    pub fn validate_file_access(&self, path: &str, op: &str) -> ValidationResult {
        let r = self.file_access_rule.validate(path, op);
        if !r.allowed {
            log::warn!("[Policy] File access blocked: {} ({})", path, op);
        }
        r
    }

    pub fn validate_time(&self) -> ValidationResult {
        self.time_rule.validate()
    }
}
