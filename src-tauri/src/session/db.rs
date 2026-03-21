use chrono::Utc;
use rusqlite::{params, Connection, Result as SqlResult};
use std::path::Path;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use super::models::*;

/// Thread-safe wrapper around the SQLite connection for session data.
pub struct SessionDb {
    conn: Arc<Mutex<Connection>>,
}

impl SessionDb {
    /// Open (or create) the session database at the given path.
    pub fn open(db_path: &Path) -> SqlResult<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.migrate()?;
        Ok(db)
    }

    // ─── Schema migration ───────────────────────────────────────────────

    fn migrate(&self) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS sessions (
                id              TEXT PRIMARY KEY,
                code            TEXT UNIQUE NOT NULL,
                name            TEXT NOT NULL,
                created_by      TEXT NOT NULL,
                mode            TEXT NOT NULL DEFAULT 'lan',
                status          TEXT NOT NULL DEFAULT 'created',
                duration_minutes INTEGER NOT NULL,
                starts_at       TEXT,
                ends_at         TEXT,
                allowed_urls    TEXT DEFAULT '[]',
                policy_json     TEXT DEFAULT '{}',
                options_json    TEXT DEFAULT '{}',
                created_at      TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS session_questions (
                id              TEXT PRIMARY KEY,
                session_id      TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                title           TEXT NOT NULL,
                description     TEXT NOT NULL DEFAULT '',
                input_data      TEXT,
                expected_output TEXT,
                time_limit_ms   INTEGER DEFAULT 5000,
                sort_order      INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS participants (
                id              TEXT PRIMARY KEY,
                session_id      TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                student_id      TEXT NOT NULL,
                display_name    TEXT,
                state           TEXT NOT NULL DEFAULT 'joined',
                last_seen_at    TEXT,
                joined_at       TEXT NOT NULL,
                submitted_at    TEXT,
                UNIQUE(session_id, student_id)
            );

            CREATE TABLE IF NOT EXISTS submissions (
                id              TEXT PRIMARY KEY,
                session_id      TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                student_id      TEXT NOT NULL,
                filename        TEXT NOT NULL,
                content         TEXT NOT NULL,
                lang            TEXT,
                is_final        INTEGER DEFAULT 0,
                judge_result    TEXT DEFAULT 'pending',
                judge_stdout    TEXT,
                judge_stderr    TEXT,
                exec_time_ms    INTEGER,
                submitted_at    TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS violations (
                id              TEXT PRIMARY KEY,
                session_id      TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                student_id      TEXT NOT NULL,
                event_type      TEXT NOT NULL,
                severity        TEXT DEFAULT 'warning',
                details         TEXT,
                occurred_at     TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS broadcasts (
                id              TEXT PRIMARY KEY,
                session_id      TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                sender_id       TEXT NOT NULL,
                content         TEXT NOT NULL,
                target_type     TEXT DEFAULT 'all',
                target_ids      TEXT,
                created_at      TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS broadcast_receipts (
                id              TEXT PRIMARY KEY,
                broadcast_id    TEXT NOT NULL REFERENCES broadcasts(id) ON DELETE CASCADE,
                student_id      TEXT NOT NULL,
                delivered_at    TEXT,
                acknowledged_at TEXT,
                UNIQUE(broadcast_id, student_id)
            );
            ",
        )?;
        Ok(())
    }

    // ─── Session CRUD ───────────────────────────────────────────────────

    pub fn create_session(
        &self,
        name: &str,
        created_by: &str,
        mode: &str,
        duration_minutes: u32,
        allowed_urls: &[String],
        options: &SessionOptions,
    ) -> SqlResult<Session> {
        let id = Uuid::new_v4().to_string();
        let code = generate_session_code();
        let now = Utc::now();
        let urls_json = serde_json::to_string(allowed_urls).unwrap_or_else(|_| "[]".into());
        let opts_json = serde_json::to_string(options).unwrap_or_else(|_| "{}".into());

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sessions (id, code, name, created_by, mode, status, duration_minutes,
             allowed_urls, policy_json, options_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'created', ?6, ?7, '{}', ?8, ?9)",
            params![id, code, name, created_by, mode, duration_minutes, urls_json, opts_json, now.to_rfc3339()],
        )?;

        Ok(Session {
            id,
            code,
            name: name.to_string(),
            created_by: created_by.to_string(),
            mode: if mode == "online" { SessionMode::Online } else { SessionMode::Lan },
            status: SessionStatus::Created,
            duration_minutes,
            starts_at: None,
            ends_at: None,
            allowed_urls: allowed_urls.to_vec(),
            policy_json: "{}".into(),
            options: options.clone(),
            created_at: now,
        })
    }

    pub fn get_session_by_code(&self, code: &str) -> SqlResult<Option<Session>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, code, name, created_by, mode, status, duration_minutes,
                    starts_at, ends_at, allowed_urls, policy_json, options_json, created_at
             FROM sessions WHERE code = ?1",
        )?;

        let mut rows = stmt.query(params![code])?;
        match rows.next()? {
            Some(row) => Ok(Some(row_to_session(row)?)),
            None => Ok(None),
        }
    }

    pub fn get_session_by_id(&self, id: &str) -> SqlResult<Option<Session>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, code, name, created_by, mode, status, duration_minutes,
                    starts_at, ends_at, allowed_urls, policy_json, options_json, created_at
             FROM sessions WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(row_to_session(row)?)),
            None => Ok(None),
        }
    }

    pub fn start_session(&self, session_id: &str) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        // Read duration to compute ends_at
        let duration: u32 = conn.query_row(
            "SELECT duration_minutes FROM sessions WHERE id = ?1",
            params![session_id],
            |row| row.get(0),
        )?;
        let now = Utc::now();
        let ends_at = now + chrono::Duration::minutes(duration as i64);
        conn.execute(
            "UPDATE sessions SET status = 'active', starts_at = ?1, ends_at = ?2 WHERE id = ?3",
            params![now.to_rfc3339(), ends_at.to_rfc3339(), session_id],
        )?;
        Ok(())
    }

    pub fn end_session(&self, session_id: &str) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE sessions SET status = 'ended' WHERE id = ?1",
            params![session_id],
        )?;
        Ok(())
    }

    pub fn delete_session(&self, session_id: &str) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        // CASCADE handles child rows
        conn.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])?;
        Ok(())
    }

    pub fn list_sessions(&self) -> SqlResult<Vec<Session>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, code, name, created_by, mode, status, duration_minutes,
                    starts_at, ends_at, allowed_urls, policy_json, options_json, created_at
             FROM sessions ORDER BY created_at DESC",
        )?;
        let sessions = stmt
            .query_map([], |row| row_to_session(row))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(sessions)
    }

    // ─── Questions ──────────────────────────────────────────────────────

    pub fn add_question(
        &self,
        session_id: &str,
        title: &str,
        description: &str,
        input_data: Option<&str>,
        expected_output: Option<&str>,
        time_limit_ms: u32,
        order: u32,
    ) -> SqlResult<SessionQuestion> {
        let id = Uuid::new_v4().to_string();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO session_questions (id, session_id, title, description, input_data,
             expected_output, time_limit_ms, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, session_id, title, description, input_data, expected_output, time_limit_ms, order],
        )?;
        Ok(SessionQuestion {
            id,
            session_id: session_id.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            input_data: input_data.map(String::from),
            expected_output: expected_output.map(String::from),
            time_limit_ms,
            order,
        })
    }

    pub fn get_questions(&self, session_id: &str) -> SqlResult<Vec<SessionQuestion>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, title, description, input_data, expected_output,
                    time_limit_ms, sort_order
             FROM session_questions WHERE session_id = ?1 ORDER BY sort_order",
        )?;
        let qs = stmt
            .query_map(params![session_id], |row| {
                Ok(SessionQuestion {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    input_data: row.get(4)?,
                    expected_output: row.get(5)?,
                    time_limit_ms: row.get(6)?,
                    order: row.get(7)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(qs)
    }

    // ─── Participants ───────────────────────────────────────────────────

    pub fn add_participant(
        &self,
        session_id: &str,
        student_id: &str,
        display_name: Option<&str>,
    ) -> SqlResult<Participant> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO participants (id, session_id, student_id, display_name, state, joined_at, last_seen_at)
             VALUES (?1, ?2, ?3, ?4, 'joined', ?5, ?5)",
            params![id, session_id, student_id, display_name, now.to_rfc3339()],
        )?;
        Ok(Participant {
            id,
            session_id: session_id.to_string(),
            student_id: student_id.to_string(),
            display_name: display_name.map(String::from),
            state: ParticipantState::Joined,
            last_seen_at: Some(now),
            joined_at: now,
            submitted_at: None,
        })
    }

    pub fn get_participant(
        &self,
        session_id: &str,
        student_id: &str,
    ) -> SqlResult<Option<Participant>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, student_id, display_name, state, last_seen_at, joined_at, submitted_at
             FROM participants WHERE session_id = ?1 AND student_id = ?2",
        )?;
        let mut rows = stmt.query(params![session_id, student_id])?;
        match rows.next()? {
            Some(row) => Ok(Some(row_to_participant(row)?)),
            None => Ok(None),
        }
    }

    pub fn get_participants(&self, session_id: &str) -> SqlResult<Vec<Participant>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, student_id, display_name, state, last_seen_at, joined_at, submitted_at
             FROM participants WHERE session_id = ?1",
        )?;
        let ps = stmt
            .query_map(params![session_id], |row| row_to_participant(row))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(ps)
    }

    pub fn update_participant_state(
        &self,
        session_id: &str,
        student_id: &str,
        state: &str,
    ) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE participants SET state = ?1 WHERE session_id = ?2 AND student_id = ?3",
            params![state, session_id, student_id],
        )?;
        Ok(())
    }

    pub fn update_heartbeat(
        &self,
        session_id: &str,
        student_id: &str,
    ) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE participants SET last_seen_at = ?1 WHERE session_id = ?2 AND student_id = ?3",
            params![now, session_id, student_id],
        )?;
        Ok(())
    }

    // ─── Submissions ────────────────────────────────────────────────────

    pub fn add_submission(
        &self,
        session_id: &str,
        student_id: &str,
        filename: &str,
        content: &str,
        lang: Option<&str>,
        is_final: bool,
    ) -> SqlResult<Submission> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO submissions (id, session_id, student_id, filename, content, lang,
             is_final, judge_result, submitted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8)",
            params![id, session_id, student_id, filename, content, lang,
                    is_final as i32, now.to_rfc3339()],
        )?;

        if is_final {
            conn.execute(
                "UPDATE participants SET state = 'submitted', submitted_at = ?1
                 WHERE session_id = ?2 AND student_id = ?3",
                params![now.to_rfc3339(), session_id, student_id],
            )?;
        }

        Ok(Submission {
            id,
            session_id: session_id.to_string(),
            student_id: student_id.to_string(),
            filename: filename.to_string(),
            content: content.to_string(),
            lang: lang.map(String::from),
            is_final,
            judge_result: JudgeResult::Pending,
            judge_stdout: None,
            judge_stderr: None,
            exec_time_ms: None,
            submitted_at: now,
        })
    }

    pub fn get_submissions(&self, session_id: &str) -> SqlResult<Vec<Submission>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, student_id, filename, content, lang, is_final,
                    judge_result, judge_stdout, judge_stderr, exec_time_ms, submitted_at
             FROM submissions WHERE session_id = ?1 ORDER BY submitted_at DESC",
        )?;
        let subs = stmt
            .query_map(params![session_id], |row| row_to_submission(row))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(subs)
    }

    pub fn get_final_submissions(&self, session_id: &str) -> SqlResult<Vec<Submission>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, student_id, filename, content, lang, is_final,
                    judge_result, judge_stdout, judge_stderr, exec_time_ms, submitted_at
             FROM submissions WHERE session_id = ?1 AND is_final = 1 ORDER BY submitted_at DESC",
        )?;
        let subs = stmt
            .query_map(params![session_id], |row| row_to_submission(row))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(subs)
    }

    pub fn update_submission_result(
        &self,
        submission_id: &str,
        judge_result: &str,
        judge_stdout: Option<&str>,
        judge_stderr: Option<&str>,
        exec_time_ms: Option<u32>,
    ) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE submissions SET judge_result = ?1, judge_stdout = ?2,
             judge_stderr = ?3, exec_time_ms = ?4 WHERE id = ?5",
            params![judge_result, judge_stdout, judge_stderr, exec_time_ms, submission_id],
        )?;
        Ok(())
    }

    // ─── Violations ─────────────────────────────────────────────────────

    pub fn add_violation(
        &self,
        session_id: &str,
        student_id: &str,
        event_type: &str,
        severity: &str,
        details: Option<&str>,
    ) -> SqlResult<Violation> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO violations (id, session_id, student_id, event_type, severity, details, occurred_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, session_id, student_id, event_type, severity, details, now.to_rfc3339()],
        )?;
        Ok(Violation {
            id,
            session_id: session_id.to_string(),
            student_id: student_id.to_string(),
            event_type: event_type.to_string(),
            severity: severity.to_string(),
            details: details.map(String::from),
            occurred_at: now,
        })
    }

    pub fn get_violations(&self, session_id: &str) -> SqlResult<Vec<Violation>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, student_id, event_type, severity, details, occurred_at
             FROM violations WHERE session_id = ?1 ORDER BY occurred_at DESC",
        )?;
        let vs = stmt
            .query_map(params![session_id], |row| {
                Ok(Violation {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    student_id: row.get(2)?,
                    event_type: row.get(3)?,
                    severity: row.get(4)?,
                    details: row.get(5)?,
                    occurred_at: parse_dt_or_now(row.get::<_, Option<String>>(6)?),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(vs)
    }

    // ─── Broadcasts ─────────────────────────────────────────────────────

    pub fn add_broadcast(
        &self,
        session_id: &str,
        sender_id: &str,
        content: &str,
        target_type: &str,
        target_ids: Option<&[String]>,
    ) -> SqlResult<Broadcast> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let targets_json = target_ids
            .map(|ids| serde_json::to_string(ids).unwrap_or_else(|_| "[]".into()));
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO broadcasts (id, session_id, sender_id, content, target_type, target_ids, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, session_id, sender_id, content, target_type, targets_json, now.to_rfc3339()],
        )?;

        let recipients: Vec<String> = if target_type == "specific" {
            target_ids.map(|ids| ids.to_vec()).unwrap_or_default()
        } else {
            let mut stmt = conn.prepare(
                "SELECT student_id FROM participants WHERE session_id = ?1 AND state != 'kicked'",
            )?;
            let rows = stmt.query_map(params![session_id], |row| row.get::<_, String>(0))?;
            rows
                .filter_map(|r| r.ok())
                .collect()
        };

        for student_id in recipients.iter() {
            let rid = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT OR IGNORE INTO broadcast_receipts (id, broadcast_id, student_id)
                 VALUES (?1, ?2, ?3)",
                params![rid, id, student_id],
            )?;
        }

        Ok(Broadcast {
            id,
            session_id: session_id.to_string(),
            sender_id: sender_id.to_string(),
            content: content.to_string(),
            target_type: if target_type == "specific" {
                BroadcastTarget::Specific
            } else {
                BroadcastTarget::All
            },
            target_ids: target_ids.map(|ids| ids.to_vec()),
            created_at: now,
        })
    }

    pub fn get_broadcasts(&self, session_id: &str) -> SqlResult<Vec<Broadcast>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, sender_id, content, target_type, target_ids, created_at
             FROM broadcasts WHERE session_id = ?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt
            .query_map(params![session_id], |row| {
                let target_type: String = row.get(4)?;
                let target_ids_str: Option<String> = row.get(5)?;
                let target_ids = target_ids_str
                    .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok());
                Ok(Broadcast {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    sender_id: row.get(2)?,
                    content: row.get(3)?,
                    target_type: if target_type == "specific" {
                        BroadcastTarget::Specific
                    } else {
                        BroadcastTarget::All
                    },
                    target_ids,
                    created_at: parse_dt_or_now(row.get::<_, Option<String>>(6)?),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    pub fn get_student_broadcasts(
        &self,
        session_id: &str,
        student_id: &str,
    ) -> SqlResult<Vec<Broadcast>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT b.id, b.session_id, b.sender_id, b.content, b.target_type, b.target_ids, b.created_at
             FROM broadcasts b
             INNER JOIN broadcast_receipts r ON r.broadcast_id = b.id
             WHERE b.session_id = ?1 AND r.student_id = ?2
             ORDER BY b.created_at DESC",
        )?;
        let rows = stmt
            .query_map(params![session_id, student_id], |row| {
                let target_type: String = row.get(4)?;
                let target_ids_str: Option<String> = row.get(5)?;
                let target_ids = target_ids_str
                    .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok());
                Ok(Broadcast {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    sender_id: row.get(2)?,
                    content: row.get(3)?,
                    target_type: if target_type == "specific" {
                        BroadcastTarget::Specific
                    } else {
                        BroadcastTarget::All
                    },
                    target_ids,
                    created_at: parse_dt_or_now(row.get::<_, Option<String>>(6)?),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    pub fn get_broadcast_receipts(&self, session_id: &str) -> SqlResult<Vec<BroadcastReceipt>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT r.id, r.broadcast_id, r.student_id, r.delivered_at, r.acknowledged_at
             FROM broadcast_receipts r
             INNER JOIN broadcasts b ON b.id = r.broadcast_id
             WHERE b.session_id = ?1",
        )?;
        let rows = stmt
            .query_map(params![session_id], |row| {
                Ok(BroadcastReceipt {
                    id: row.get(0)?,
                    broadcast_id: row.get(1)?,
                    student_id: row.get(2)?,
                    delivered_at: row.get::<_, Option<String>>(3)?.and_then(|s| s.parse().ok()),
                    acknowledged_at: row.get::<_, Option<String>>(4)?.and_then(|s| s.parse().ok()),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    pub fn mark_broadcast_delivered(&self, broadcast_id: &str, student_id: &str) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE broadcast_receipts
             SET delivered_at = COALESCE(delivered_at, ?1)
             WHERE broadcast_id = ?2 AND student_id = ?3",
            params![now, broadcast_id, student_id],
        )?;
        Ok(())
    }

    pub fn acknowledge_broadcast(&self, broadcast_id: &str, student_id: &str) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE broadcast_receipts
             SET delivered_at = COALESCE(delivered_at, ?1), acknowledged_at = ?1
             WHERE broadcast_id = ?2 AND student_id = ?3",
            params![now, broadcast_id, student_id],
        )?;
        Ok(())
    }

    // ─── Stats helpers ──────────────────────────────────────────────────

    pub fn count_submissions(&self, session_id: &str) -> SqlResult<usize> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM submissions WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    pub fn count_violations(&self, session_id: &str) -> SqlResult<usize> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM violations WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }
}

// ─── Row mapping helpers ────────────────────────────────────────────────────

fn row_to_session(row: &rusqlite::Row) -> SqlResult<Session> {
    let mode_str: String = row.get(4)?;
    let status_str: String = row.get(5)?;
    let urls_str: Option<String> = row.get(9)?;
    let opts_str: Option<String> = row.get(11)?;

    Ok(Session {
        id: row.get(0)?,
        code: row.get(1)?,
        name: row.get(2)?,
        created_by: row.get(3)?,
        mode: if mode_str == "online" { SessionMode::Online } else { SessionMode::Lan },
        status: match status_str.as_str() {
            "active" => SessionStatus::Active,
            "ended" => SessionStatus::Ended,
            _ => SessionStatus::Created,
        },
        duration_minutes: row.get(6)?,
        starts_at: row.get::<_, Option<String>>(7)?.and_then(|s| s.parse().ok()),
        ends_at: row.get::<_, Option<String>>(8)?.and_then(|s| s.parse().ok()),
        allowed_urls: urls_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default(),
        policy_json: row.get(10)?,
        options: opts_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default(),
        created_at: parse_dt_or_now(row.get::<_, Option<String>>(12)?),
    })
}

fn row_to_participant(row: &rusqlite::Row) -> SqlResult<Participant> {
    let state_str: String = row.get(4)?;
    Ok(Participant {
        id: row.get(0)?,
        session_id: row.get(1)?,
        student_id: row.get(2)?,
        display_name: row.get(3)?,
        state: match state_str.as_str() {
            "active" => ParticipantState::Active,
            "submitted" => ParticipantState::Submitted,
            "kicked" => ParticipantState::Kicked,
            "disconnected" => ParticipantState::Disconnected,
            _ => ParticipantState::Joined,
        },
        last_seen_at: row.get::<_, Option<String>>(5)?.and_then(|s| s.parse().ok()),
        joined_at: parse_dt_or_now(row.get::<_, Option<String>>(6)?),
        submitted_at: row.get::<_, Option<String>>(7)?.and_then(|s| s.parse().ok()),
    })
}

fn row_to_submission(row: &rusqlite::Row) -> SqlResult<Submission> {
    let result_str: String = row.get(7)?;
    let is_final_int: i32 = row.get(6)?;
    Ok(Submission {
        id: row.get(0)?,
        session_id: row.get(1)?,
        student_id: row.get(2)?,
        filename: row.get(3)?,
        content: row.get(4)?,
        lang: row.get(5)?,
        is_final: is_final_int != 0,
        judge_result: match result_str.as_str() {
            "pass" => JudgeResult::Pass,
            "partial" => JudgeResult::Partial,
            "fail" => JudgeResult::Fail,
            "compile_error" => JudgeResult::CompileError,
            "timeout" => JudgeResult::Timeout,
            _ => JudgeResult::Pending,
        },
        judge_stdout: row.get(8)?,
        judge_stderr: row.get(9)?,
        exec_time_ms: row.get(10)?,
        submitted_at: parse_dt_or_now(row.get::<_, Option<String>>(11)?),
    })
}

fn parse_dt_or_now(s: Option<String>) -> chrono::DateTime<Utc> {
    s.and_then(|v| v.parse().ok()).unwrap_or_else(Utc::now)
}

/// Generate a 6-character alphanumeric session code (uppercase).
fn generate_session_code() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::SystemTime;

    let mut hasher = DefaultHasher::new();
    SystemTime::now().hash(&mut hasher);
    Uuid::new_v4().hash(&mut hasher);
    let hash = hasher.finish();

    const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    (0..6)
        .map(|i| {
            let idx = ((hash >> (i * 5)) & 0x1F) as usize % CHARS.len();
            CHARS[idx] as char
        })
        .collect()
}
