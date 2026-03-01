use std::time::Instant;

use crate::config::AppConfig;

// ─── Auth result ────────────────────────────────────────────────────────────

pub struct AuthResult {
    pub success: bool,
    pub error: Option<String>,
    pub attempts_remaining: u32,
}

// ─── Session manager ────────────────────────────────────────────────────────

pub struct SessionManager {
    /// bcrypt hash of the admin password.
    password_hash: String,
    /// Whether a session is currently active.
    authenticated: bool,
    /// When the current session was started.
    session_start: Option<Instant>,
    /// Session timeout (ms).
    timeout_ms: u64,
    /// Failed login attempts since last success.
    failed_attempts: u32,
    max_attempts: u32,
    /// Lockout timestamp.
    lockout_until: Option<Instant>,
    lockout_duration_ms: u64,
}

impl SessionManager {
    pub fn new(config: &AppConfig) -> Self {
        // Hash the default admin password ("admin123")
        let default_password = std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin123".into());
        let hash = bcrypt::hash(&default_password, 12).expect("bcrypt hash failed");

        log::info!("[Session] SessionManager initialised (timeout={}ms)", config.admin.session_timeout_ms);

        Self {
            password_hash: hash,
            authenticated: false,
            session_start: None,
            timeout_ms: config.admin.session_timeout_ms,
            failed_attempts: 0,
            max_attempts: config.admin.max_login_attempts,
            lockout_until: None,
            lockout_duration_ms: config.admin.lockout_duration_ms,
        }
    }

    // ── Authentication ──

    pub fn authenticate(&mut self, password: &str) -> AuthResult {
        // Check lockout
        if let Some(until) = self.lockout_until {
            if until.elapsed().as_millis() < self.lockout_duration_ms as u128 {
                let remaining = self.lockout_duration_ms as u128 - until.elapsed().as_millis();
                return AuthResult {
                    success: false,
                    error: Some(format!(
                        "Account locked. Try again in {} seconds.",
                        remaining / 1000
                    )),
                    attempts_remaining: 0,
                };
            }
            // Lockout expired
            self.lockout_until = None;
            self.failed_attempts = 0;
        }

        // Verify
        let valid = bcrypt::verify(password, &self.password_hash).unwrap_or(false);

        if valid {
            self.authenticated = true;
            self.session_start = Some(Instant::now());
            self.failed_attempts = 0;
            log::info!("[SECURITY] Admin authenticated");
            AuthResult {
                success: true,
                error: None,
                attempts_remaining: self.max_attempts,
            }
        } else {
            self.failed_attempts += 1;
            let remaining = self.max_attempts.saturating_sub(self.failed_attempts);

            log::warn!(
                "[SECURITY] Admin login failed ({}/{})",
                self.failed_attempts,
                self.max_attempts
            );

            if self.failed_attempts >= self.max_attempts {
                self.lockout_until = Some(Instant::now());
                return AuthResult {
                    success: false,
                    error: Some(format!(
                        "Too many failed attempts. Locked for {} seconds.",
                        self.lockout_duration_ms / 1000
                    )),
                    attempts_remaining: 0,
                };
            }

            AuthResult {
                success: false,
                error: Some("Invalid password".into()),
                attempts_remaining: remaining,
            }
        }
    }

    // ── Session state ──

    pub fn is_authenticated(&self) -> bool {
        if !self.authenticated {
            return false;
        }
        // Check timeout
        if let Some(start) = self.session_start {
            if start.elapsed().as_millis() > self.timeout_ms as u128 {
                return false;
            }
        }
        true
    }

    pub fn logout(&mut self) {
        self.authenticated = false;
        self.session_start = None;
        log::info!("[SECURITY] Admin logged out");
    }
}
