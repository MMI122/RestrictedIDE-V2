use std::collections::HashMap;

use super::engine::ValidationResult;

/// Keyboard shortcut validation.
pub struct KeyboardRule {
    mode: String, // "blacklist" | "whitelist"
    blocked: HashMap<String, String>, // normalised combo → reason
}

impl KeyboardRule {
    pub fn new(mode: &str, combos: Vec<(Vec<String>, String)>) -> Self {
        let mut blocked = HashMap::new();
        for (keys, reason) in combos {
            let normalized = Self::normalize(&keys);
            blocked.insert(normalized, reason);
        }
        Self {
            mode: mode.into(),
            blocked,
        }
    }

    fn normalize(keys: &[String]) -> String {
        let mut sorted: Vec<String> = keys.iter().map(|k| k.to_lowercase()).collect();
        sorted.sort();
        sorted.join("+")
    }

    pub fn validate(&self, keys: &[String]) -> ValidationResult {
        if keys.is_empty() {
            return ValidationResult { allowed: true, reason: None };
        }

        let normalized = Self::normalize(keys);

        if self.mode == "blacklist" {
            if let Some(reason) = self.blocked.get(&normalized) {
                ValidationResult {
                    allowed: false,
                    reason: Some(reason.clone()),
                }
            } else {
                ValidationResult { allowed: true, reason: None }
            }
        } else {
            // whitelist
            if self.blocked.contains_key(&normalized) {
                ValidationResult { allowed: true, reason: None }
            } else {
                ValidationResult {
                    allowed: false,
                    reason: Some("Key combo not in whitelist".into()),
                }
            }
        }
    }
}
