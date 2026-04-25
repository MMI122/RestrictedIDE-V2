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

#[cfg(test)]
mod tests {
    use super::KeyboardRule;

    #[test]
    fn blacklist_blocks_registered_combo_order_insensitive() {
        let rule = KeyboardRule::new(
            "blacklist",
            vec![(vec!["ctrl".into(), "shift".into(), "i".into()], "blocked".into())],
        );

        let result = rule.validate(&["I".into(), "CTRL".into(), "Shift".into()]);
        assert!(!result.allowed);
        assert_eq!(result.reason.as_deref(), Some("blocked"));
    }

    #[test]
    fn blacklist_allows_unlisted_combo() {
        let rule = KeyboardRule::new(
            "blacklist",
            vec![(vec!["alt".into(), "f4".into()], "no-close".into())],
        );

        let result = rule.validate(&["ctrl".into(), "s".into()]);
        assert!(result.allowed);
        assert!(result.reason.is_none());
    }

    #[test]
    fn whitelist_rejects_unknown_combo() {
        let rule = KeyboardRule::new(
            "whitelist",
            vec![(vec!["ctrl".into(), "s".into()], "save".into())],
        );

        let result = rule.validate(&["ctrl".into(), "p".into()]);
        assert!(!result.allowed);
        assert_eq!(result.reason.as_deref(), Some("Key combo not in whitelist"));
    }
}
