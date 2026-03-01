use std::collections::HashSet;
use std::path::Path;

use super::engine::ValidationResult;

/// Process validation (whitelist / blacklist with system-process exceptions).
pub struct ProcessRule {
    mode: String,
    allowed: HashSet<String>,
    blocked: HashSet<String>,
    system_processes: HashSet<String>,
}

impl ProcessRule {
    pub fn new(mode: &str, allowed: Vec<String>, blocked: Vec<String>) -> Self {
        let system_processes: HashSet<String> = [
            "system", "idle", "csrss.exe", "smss.exe", "services.exe", "lsass.exe",
            "svchost.exe", "conhost.exe", "dwm.exe", "winlogon.exe", "wininit.exe",
        ]
        .iter()
        .map(|s| s.to_lowercase())
        .collect();

        Self {
            mode: mode.into(),
            allowed: allowed.into_iter().map(|s| s.to_lowercase()).collect(),
            blocked: blocked.into_iter().map(|s| s.to_lowercase()).collect(),
            system_processes,
        }
    }

    pub fn validate(&self, process_name: &str) -> ValidationResult {
        if process_name.is_empty() {
            return ValidationResult {
                allowed: false,
                reason: Some("Empty process name".into()),
            };
        }

        let exe = Path::new(process_name)
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_else(|| process_name.to_lowercase());

        // System processes are always allowed
        if self.system_processes.contains(&exe) {
            return ValidationResult { allowed: true, reason: Some("System process".into()) };
        }

        // Blacklist check
        if self.blocked.contains(&exe) {
            return ValidationResult {
                allowed: false,
                reason: Some(format!("Process blocked: {}", exe)),
            };
        }

        if self.mode == "whitelist" {
            if self.allowed.contains(&exe) {
                ValidationResult { allowed: true, reason: None }
            } else {
                ValidationResult {
                    allowed: false,
                    reason: Some(format!("Process not in whitelist: {}", exe)),
                }
            }
        } else {
            ValidationResult { allowed: true, reason: None }
        }
    }

    pub fn should_terminate(&self, name: &str) -> bool {
        !self.validate(name).allowed
    }
}
