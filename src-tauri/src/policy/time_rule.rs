use chrono::Local;
use serde::{Deserialize, Serialize};

use super::engine::ValidationResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub start_time: String, // "HH:MM"
    pub end_time: String,   // "HH:MM"
    pub days: Vec<u32>,     // 0 = Sunday … 6 = Saturday
}

/// Time-based access restrictions.
pub struct TimeRule {
    enabled: bool,
    schedule: Option<Schedule>,
}

impl TimeRule {
    pub fn new(enabled: bool, schedule: Option<Schedule>) -> Self {
        Self { enabled, schedule }
    }

    pub fn validate(&self) -> ValidationResult {
        if !self.enabled {
            return ValidationResult { allowed: true, reason: None };
        }

        let schedule = match &self.schedule {
            Some(s) => s,
            None => return ValidationResult { allowed: true, reason: None },
        };

        let now = Local::now();
        let day: u32 = now.format("%w").to_string().parse().unwrap_or(0);
        let time = now.format("%H:%M").to_string();

        // Day check
        if !schedule.days.is_empty() && !schedule.days.contains(&day) {
            return ValidationResult {
                allowed: false,
                reason: Some(format!("Not available on day {}", day)),
            };
        }

        // Time-window check
        if time < schedule.start_time || time > schedule.end_time {
            return ValidationResult {
                allowed: false,
                reason: Some(format!(
                    "Outside allowed time ({} – {})",
                    schedule.start_time, schedule.end_time
                )),
            };
        }

        ValidationResult { allowed: true, reason: None }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Local;

    use super::{Schedule, TimeRule};

    #[test]
    fn disabled_rule_always_allows() {
        let rule = TimeRule::new(true, None);
        let result = rule.validate();
        assert!(result.allowed);
    }

    #[test]
    fn allows_when_current_time_is_inside_window_for_today() {
        let today: u32 = Local::now().format("%w").to_string().parse().unwrap_or(0);
        let schedule = Schedule {
            start_time: "00:00".to_string(),
            end_time: "23:59".to_string(),
            days: vec![today],
        };
        let rule = TimeRule::new(true, Some(schedule));

        let result = rule.validate();
        assert!(result.allowed);
    }

    #[test]
    fn denies_when_today_is_not_in_allowed_days() {
        let today: u32 = Local::now().format("%w").to_string().parse().unwrap_or(0);
        let different_day = (today + 1) % 7;
        let schedule = Schedule {
            start_time: "00:00".to_string(),
            end_time: "23:59".to_string(),
            days: vec![different_day],
        };
        let rule = TimeRule::new(true, Some(schedule));

        let result = rule.validate();
        assert!(!result.allowed);
        assert!(result.reason.unwrap_or_default().contains("Not available on day"));
    }
}
