use super::db::SessionDb;
use super::models::*;
use std::sync::Arc;

/// Abstraction over the networking layer.
/// Both LAN (embedded axum) and Online (Supabase) implement this interface.
/// The UI code only interacts through this trait — never knows which transport is active.
pub trait SessionTransport: Send + Sync {
    /// Create a new session (admin side). Returns the session + server address.
    fn create_session(
        &self,
        req: CreateSessionRequest,
        admin_id: &str,
    ) -> Result<CreateSessionResponse, TransportError>;

    /// Start an already-created session (activates timer).
    fn start_session(&self, session_id: &str) -> Result<(), TransportError>;

    /// End a session (admin side).
    fn end_session(&self, session_id: &str) -> Result<(), TransportError>;

    /// Permanently delete a session and all associated data.
    fn delete_session(&self, session_id: &str) -> Result<(), TransportError>;

    /// Cleanup remote/storage assets associated with a session before deletion.
    ///
    /// LAN transport is expected to no-op. Online transport can remove
    /// Supabase storage objects or other external artifacts.
    fn cleanup_remote_assets(&self, session_id: &str) -> Result<(), TransportError>;

    /// Get full session status (admin dashboard).
    fn get_session_status(&self, session_id: &str) -> Result<SessionStatusResponse, TransportError>;

    /// List all sessions (admin).
    fn list_sessions(&self) -> Result<Vec<Session>, TransportError>;

    /// Student joins a session.
    fn join_session(
        &self,
        server_addr: &str,
        code: &str,
        student_id: &str,
        display_name: Option<&str>,
        device_id: Option<&str>,
    ) -> Result<JoinSessionResponse, TransportError>;

    /// Student submits code.
    fn submit_code(&self, req: SubmitCodeRequest) -> Result<Submission, TransportError>;

    /// Student heartbeat (keep-alive).
    fn heartbeat(&self, req: HeartbeatRequest) -> Result<(), TransportError>;

    /// Admin sends a broadcast message.
    fn broadcast(&self, req: BroadcastRequest, sender_id: &str) -> Result<Broadcast, TransportError>;

    /// Admin kicks a participant.
    fn kick_participant(
        &self,
        session_id: &str,
        student_id: &str,
    ) -> Result<(), TransportError>;

    /// Get all submissions for a session.
    fn get_submissions(&self, session_id: &str) -> Result<Vec<Submission>, TransportError>;

    /// Get all violations for a session.
    fn get_violations(&self, session_id: &str) -> Result<Vec<Violation>, TransportError>;

    /// Get all participants for a session.
    fn get_participants(&self, session_id: &str) -> Result<Vec<Participant>, TransportError>;

    /// Get questions for a session.
    #[allow(dead_code)] // used by upcoming online sync/reload flows
    fn get_questions(&self, session_id: &str) -> Result<Vec<SessionQuestion>, TransportError>;
}

/// Transport-level errors. Serializable so we can send them over IPC.
#[allow(dead_code)] // some variants are for OnlineTransport and not all are used in LAN mode yet
#[derive(Debug)]
pub enum TransportError {
    NotFound(String),
    AlreadyExists(String),
    InvalidState(String),
    Database(String),
    Network(String),
    Internal(String),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "Not found: {}", msg),
            Self::AlreadyExists(msg) => write!(f, "Already exists: {}", msg),
            Self::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
            Self::Database(msg) => write!(f, "Database error: {}", msg),
            Self::Network(msg) => write!(f, "Network error: {}", msg),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl From<rusqlite::Error> for TransportError {
    fn from(e: rusqlite::Error) -> Self {
        TransportError::Database(e.to_string())
    }
}

// ─── LAN Transport (embedded, uses SessionDb directly) ──────────────────────

pub struct LanTransport {
    pub db: Arc<SessionDb>,
}

impl LanTransport {
    pub fn new(db: Arc<SessionDb>) -> Self {
        Self { db }
    }

    fn normalize_device_id(device_id: Option<&str>) -> Option<String> {
        let trimmed = device_id.map(str::trim).unwrap_or_default();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    fn find_locked_device_owner(
        &self,
        session_id: &str,
        student_id: &str,
        device_id: &str,
    ) -> Result<Option<Participant>, TransportError> {
        let participants = self.db.get_participants(session_id)?;
        Ok(participants.into_iter().find(|p| {
            p.student_id != student_id
                && p.device_id.as_deref() == Some(device_id)
                && matches!(
                    p.state,
                    ParticipantState::Submitted
                        | ParticipantState::Kicked
                        | ParticipantState::ReentryPending
                )
        }))
    }
}

impl SessionTransport for LanTransport {
    fn create_session(
        &self,
        req: CreateSessionRequest,
        admin_id: &str,
    ) -> Result<CreateSessionResponse, TransportError> {
        let session = self.db.create_session(
            &req.name,
            admin_id,
            "lan",
            req.duration_minutes,
            &req.allowed_urls,
            &req.options,
        )?;

        // Add questions
        for (i, q) in req.questions.iter().enumerate() {
            self.db.add_question(
                &session.id,
                &q.title,
                &q.description,
                q.input_data.as_deref(),
                q.expected_output.as_deref(),
                &q.visible_testcases,
                &q.hidden_testcases,
                q.time_limit_ms.unwrap_or(5000),
                i as u32,
            )?;
        }

        Ok(CreateSessionResponse {
            session_id: session.id,
            code: session.code,
            server_addr: String::new(), // filled by the caller with actual IP:port
        })
    }

    fn start_session(&self, session_id: &str) -> Result<(), TransportError> {
        let session = self
            .db
            .get_session_by_id(session_id)?
            .ok_or_else(|| TransportError::NotFound("Session not found".into()))?;
        if session.status != SessionStatus::Created {
            return Err(TransportError::InvalidState(
                "Session is not in 'created' state".into(),
            ));
        }
        self.db.start_session(session_id)?;
        Ok(())
    }

    fn end_session(&self, session_id: &str) -> Result<(), TransportError> {
        self.db.end_session(session_id)?;
        Ok(())
    }

    fn delete_session(&self, session_id: &str) -> Result<(), TransportError> {
        self.db.delete_session(session_id)?;
        Ok(())
    }

    fn cleanup_remote_assets(&self, _session_id: &str) -> Result<(), TransportError> {
        // LAN mode has no external object storage to cleanup.
        Ok(())
    }

    fn get_session_status(&self, session_id: &str) -> Result<SessionStatusResponse, TransportError> {
        let session = self
            .db
            .get_session_by_id(session_id)?
            .ok_or_else(|| TransportError::NotFound("Session not found".into()))?;
        let participants = self.db.get_participants(session_id)?;
        let sub_count = self.db.count_submissions(session_id)?;
        let vio_count = self.db.count_violations(session_id)?;
        Ok(SessionStatusResponse {
            session,
            participants,
            submission_count: sub_count,
            violation_count: vio_count,
        })
    }

    fn list_sessions(&self) -> Result<Vec<Session>, TransportError> {
        Ok(self.db.list_sessions()?)
    }

    fn join_session(
        &self,
        _server_addr: &str,
        code: &str,
        student_id: &str,
        display_name: Option<&str>,
        device_id: Option<&str>,
    ) -> Result<JoinSessionResponse, TransportError> {
        let session = self
            .db
            .get_session_by_code(code)?
            .ok_or_else(|| TransportError::NotFound("Invalid session code".into()))?;

        if session.status == SessionStatus::Ended {
            return Err(TransportError::InvalidState("Session has ended".into()));
        }

        let normalized_device = Self::normalize_device_id(device_id);

        let existing_participant = self.db.get_participant(&session.id, student_id)?;

        if let Some(ref did) = normalized_device {
            let enforce_device_lock = match existing_participant.as_ref() {
                Some(p) => !matches!(
                    p.state,
                    ParticipantState::Joined
                        | ParticipantState::Active
                        | ParticipantState::Disconnected
                ),
                None => true,
            };

            if enforce_device_lock {
                if let Some(owner) = self.find_locked_device_owner(&session.id, student_id, did)? {
                    if existing_participant.is_none() {
                        let _ = self.db.add_participant(
                            &session.id,
                            student_id,
                            display_name,
                            Some(did),
                        );
                    }
                    let _ = self
                        .db
                        .update_participant_state(&session.id, student_id, "reentry_pending");
                    let _ = self
                        .db
                        .update_participant_device_id(&session.id, student_id, Some(did));
                    let _ = self.db.add_violation(
                        &session.id,
                        student_id,
                        "reentry_request",
                        "warning",
                        Some(&format!(
                            "Device already used in this session by {} and is awaiting/needs admin release",
                            owner.student_id
                        )),
                    );
                    return Err(TransportError::InvalidState(
                        "This device is locked for this session. Administrator approval is required to continue"
                            .into(),
                    ));
                }
            }
        }

        // Check if already joined
        if let Some(p) = existing_participant {
            match p.state {
                ParticipantState::Kicked | ParticipantState::Submitted => {
                    let _ = self
                        .db
                        .update_participant_state(&session.id, student_id, "reentry_pending");
                    let _ = self.db.add_violation(
                        &session.id,
                        student_id,
                        "reentry_request",
                        "warning",
                        Some("Student requested re-entry after removal/submission"),
                    );
                    return Err(TransportError::InvalidState(
                        "Re-entry requires administrator approval for this session".into(),
                    ));
                }
                ParticipantState::ReentryPending => {
                    return Err(TransportError::InvalidState(
                        "Re-entry request is pending administrator approval".into(),
                    ));
                }
                _ => {
                    // Already joined/active/disconnected — allow reconnect.
                    if let Some(ref did) = normalized_device {
                        let _ = self
                            .db
                            .update_participant_device_id(&session.id, student_id, Some(did));
                    }
                }
            }
        } else {
            self.db
                .add_participant(&session.id, student_id, display_name, normalized_device.as_deref())?;
        }

        let questions = self.db.get_questions(&session.id)?;

        // Calculate remaining seconds
        let remaining = if let Some(ends_at) = session.ends_at {
            let diff = ends_at - chrono::Utc::now();
            diff.num_seconds().max(0) as u64
        } else {
            (session.duration_minutes as u64) * 60
        };

        Ok(JoinSessionResponse {
            session_id: session.id,
            name: session.name,
            duration_minutes: session.duration_minutes,
            questions,
            allowed_urls: session.allowed_urls,
            options: session.options,
            remaining_seconds: remaining,
        })
    }

    fn submit_code(&self, req: SubmitCodeRequest) -> Result<Submission, TransportError> {
        Ok(self.db.add_submission(
            &req.session_id,
            &req.student_id,
            &req.filename,
            &req.content,
            req.lang.as_deref(),
            true, // final submission
        )?)
    }

    fn heartbeat(&self, req: HeartbeatRequest) -> Result<(), TransportError> {
        if let Some(session) = self.db.get_session_by_id(&req.session_id)? {
            if session.status == SessionStatus::Ended {
                return Err(TransportError::InvalidState(
                    "Session has ended by administrator".into(),
                ));
            }
        }

        if let Some(p) = self.db.get_participant(&req.session_id, &req.student_id)? {
            if matches!(
                p.state,
                ParticipantState::Kicked
                    | ParticipantState::Submitted
                    | ParticipantState::ReentryPending
            ) {
                return Err(TransportError::InvalidState(
                    "You are not allowed to re-enter this session without admin approval"
                        .into(),
                ));
            }
        }

        self.db
            .update_heartbeat(&req.session_id, &req.student_id)?;
        Ok(())
    }

    fn broadcast(
        &self,
        req: BroadcastRequest,
        sender_id: &str,
    ) -> Result<Broadcast, TransportError> {
        let target_str = match &req.target_type {
            BroadcastTarget::All => "all",
            BroadcastTarget::Specific => "specific",
        };
        Ok(self.db.add_broadcast(
            &req.session_id,
            sender_id,
            &req.content,
            target_str,
            req.target_ids.as_deref(),
        )?)
    }

    fn kick_participant(
        &self,
        session_id: &str,
        student_id: &str,
    ) -> Result<(), TransportError> {
        self.db
            .update_participant_state(session_id, student_id, "kicked")?;
        Ok(())
    }

    fn get_submissions(&self, session_id: &str) -> Result<Vec<Submission>, TransportError> {
        Ok(self.db.get_submissions(session_id)?)
    }

    fn get_violations(&self, session_id: &str) -> Result<Vec<Violation>, TransportError> {
        Ok(self.db.get_violations(session_id)?)
    }

    fn get_participants(&self, session_id: &str) -> Result<Vec<Participant>, TransportError> {
        Ok(self.db.get_participants(session_id)?)
    }

    fn get_questions(&self, session_id: &str) -> Result<Vec<SessionQuestion>, TransportError> {
        Ok(self.db.get_questions(session_id)?)
    }
}
