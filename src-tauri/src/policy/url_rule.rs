use regex::Regex;

use super::engine::ValidationResult;

/// URL validation using compiled glob→regex patterns.
pub struct UrlRule {
    mode: String, // "whitelist" | "blacklist"
    patterns: Vec<Regex>,
}

impl UrlRule {
    pub fn new(mode: &str, raw_patterns: Vec<String>) -> Self {
        let patterns: Vec<Regex> = raw_patterns
            .iter()
            .filter_map(|p| {
                let re = if p.starts_with('^') {
                    p.clone()
                } else {
                    let escaped = regex::escape(p).replace(r"\*", ".*");
                    format!("^{}$", escaped)
                };
                Regex::new(&re).ok()
            })
            .collect();

        Self {
            mode: mode.into(),
            patterns,
        }
    }

    pub fn validate(&self, url: &str) -> ValidationResult {
        if url.is_empty() {
            return ValidationResult {
                allowed: false,
                reason: Some("Empty URL".into()),
            };
        }

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return ValidationResult {
                allowed: false,
                reason: Some("Protocol not allowed".into()),
            };
        }

        let matches = self.patterns.iter().any(|p| p.is_match(url));

        if self.mode == "whitelist" {
            if matches {
                ValidationResult { allowed: true, reason: None }
            } else {
                ValidationResult {
                    allowed: false,
                    reason: Some("URL not in whitelist".into()),
                }
            }
        } else {
            // blacklist
            if matches {
                ValidationResult {
                    allowed: false,
                    reason: Some("URL is blacklisted".into()),
                }
            } else {
                ValidationResult { allowed: true, reason: None }
            }
        }
    }
}
