use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::engine::ValidationResult;

/// File-system access validation (sandbox mode, extension filtering, path denial).
pub struct FileAccessRule {
    mode: String,
    sandbox_path: PathBuf,
    allowed_extensions: HashSet<String>,
    max_file_size: u64,
    denied_paths: Vec<String>,
}

impl FileAccessRule {
    pub fn new(
        mode: &str,
        sandbox_path: &str,
        allowed_extensions: Vec<String>,
        max_file_size: u64,
        denied_paths: Vec<String>,
    ) -> Self {
        Self {
            mode: mode.into(),
            sandbox_path: PathBuf::from(sandbox_path),
            allowed_extensions: allowed_extensions
                .into_iter()
                .map(|e| e.to_lowercase())
                .collect(),
            max_file_size,
            denied_paths: denied_paths
                .into_iter()
                .map(|p| p.replace('/', "\\").to_lowercase())
                .collect(),
        }
    }

    pub fn validate(&self, file_path: &str, _operation: &str) -> ValidationResult {
        if file_path.is_empty() {
            return ValidationResult {
                allowed: false,
                reason: Some("Empty path".into()),
            };
        }

        // Path-traversal detection
        if file_path.contains("..") {
            log::warn!("[Security] Path traversal attempt: {}", file_path);
            return ValidationResult {
                allowed: false,
                reason: Some("Path traversal not allowed".into()),
            };
        }

        let normalized = file_path.replace('/', "\\").to_lowercase();

        // Denied paths
        for denied in &self.denied_paths {
            if normalized.starts_with(denied) {
                return ValidationResult {
                    allowed: false,
                    reason: Some(format!("Access denied to path: {}", denied)),
                };
            }
        }

        // Extension filter (skip for directories)
        let path = Path::new(file_path);
        if path.extension().is_some() {
            let ext = format!(
                ".{}",
                path.extension().unwrap().to_string_lossy().to_lowercase()
            );
            if !self.allowed_extensions.is_empty() && !self.allowed_extensions.contains(&ext) {
                return ValidationResult {
                    allowed: false,
                    reason: Some(format!("File extension not allowed: {}", ext)),
                };
            }
        }

        // Sandbox validation
        if self.mode == "sandbox" {
            return self.validate_sandbox(&normalized);
        }

        ValidationResult { allowed: true, reason: None }
    }

    fn validate_sandbox(&self, normalized: &str) -> ValidationResult {
        if self.sandbox_path.as_os_str().is_empty() {
            return ValidationResult {
                allowed: false,
                reason: Some("Sandbox path not configured".into()),
            };
        }

        let sandbox_str = self
            .sandbox_path
            .to_string_lossy()
            .to_lowercase()
            .replace('/', "\\");

        if !normalized.starts_with(&sandbox_str) {
            return ValidationResult {
                allowed: false,
                reason: Some("Path outside sandbox".into()),
            };
        }

        ValidationResult { allowed: true, reason: None }
    }

    pub fn validate_file_size(&self, size: u64) -> ValidationResult {
        if size > self.max_file_size {
            return ValidationResult {
                allowed: false,
                reason: Some(format!(
                    "File size {} exceeds limit {}",
                    size, self.max_file_size
                )),
            };
        }
        ValidationResult { allowed: true, reason: None }
    }

    pub fn get_sandbox_path(&self) -> &Path {
        &self.sandbox_path
    }
}
