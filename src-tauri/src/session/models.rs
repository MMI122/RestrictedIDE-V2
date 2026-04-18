use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ─── Session ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub code: String,
    pub name: String,
    pub created_by: String,
    pub mode: SessionMode,
    pub status: SessionStatus,
    pub duration_minutes: u32,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub allowed_urls: Vec<String>,
    pub policy_json: String,
    pub options: SessionOptions,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SessionMode {
    Lan,
    Online,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Created,
    Active,
    Ended,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionOptions {
    pub video: bool,
    pub audio: bool,
    pub screen_share: bool,
    pub recording: bool,
    #[serde(default = "default_security_mode")]
    pub security_mode: String,
    #[serde(default = "default_true")]
    pub block_vm: bool,
    #[serde(default = "default_true")]
    pub block_multi_monitor: bool,
    #[serde(default = "default_true")]
    pub prevent_screenshots: bool,
    #[serde(default = "default_true")]
    pub focus_watchdog: bool,
    #[serde(default = "default_true")]
    pub controlled_paste: bool,
    #[serde(default = "default_focus_auto_submit_threshold")]
    pub focus_auto_submit_threshold: u32,
    #[serde(default = "default_disconnect_grace_seconds")]
    pub disconnect_grace_seconds: u32,
    #[serde(default = "default_false")]
    pub lockdown_emergency_unlock: bool,
    #[serde(default, skip_serializing)]
    pub lockdown_exit_password_hash: Option<String>,
}

fn default_disconnect_grace_seconds() -> u32 {
    120
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_security_mode() -> String {
    "monitor".to_string()
}

fn default_focus_auto_submit_threshold() -> u32 {
    3
}

impl Default for SessionOptions {
    fn default() -> Self {
        Self {
            video: false,
            audio: false,
            screen_share: false,
            recording: false,
            security_mode: default_security_mode(),
            block_vm: true,
            block_multi_monitor: true,
            prevent_screenshots: true,
            focus_watchdog: true,
            controlled_paste: true,
            focus_auto_submit_threshold: default_focus_auto_submit_threshold(),
            disconnect_grace_seconds: default_disconnect_grace_seconds(),
            lockdown_emergency_unlock: default_false(),
            lockdown_exit_password_hash: None,
        }
    }
}

// ─── Question ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionQuestion {
    pub id: String,
    pub session_id: String,
    pub title: String,
    pub description: String,
    pub input_data: Option<String>,
    pub expected_output: Option<String>,
    #[serde(default)]
    pub visible_testcases: Vec<TestCase>,
    #[serde(default, skip_serializing)]
    pub hidden_testcases: Vec<TestCase>,
    pub time_limit_ms: u32,
    pub order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub input: String,
    pub expected_output: String,
    #[serde(default)]
    pub hidden: bool,
}

// ─── Participant ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub id: String,
    pub session_id: String,
    pub student_id: String,
    pub display_name: Option<String>,
    pub device_id: Option<String>,
    pub state: ParticipantState,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub joined_at: DateTime<Utc>,
    pub submitted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantState {
    Joined,
    Active,
    Submitted,
    Kicked,
    ReentryPending,
    Disconnected,
}

// ─── Submission ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Submission {
    pub id: String,
    pub session_id: String,
    pub student_id: String,
    pub filename: String,
    pub content: String,
    pub lang: Option<String>,
    pub is_final: bool,
    pub judge_result: JudgeResult,
    pub judge_stdout: Option<String>,
    pub judge_stderr: Option<String>,
    pub exec_time_ms: Option<u32>,
    pub submitted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JudgeResult {
    Pending,
    Pass,
    Partial,
    Fail,
    CompileError,
    Timeout,
}

// ─── Violation ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub id: String,
    pub session_id: String,
    pub student_id: String,
    pub event_type: String,
    pub severity: String,
    pub details: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

// ─── Broadcast ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Broadcast {
    pub id: String,
    pub session_id: String,
    pub sender_id: String,
    pub content: String,
    pub target_type: BroadcastTarget,
    pub target_ids: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastReceipt {
    pub id: String,
    pub broadcast_id: String,
    pub student_id: String,
    pub delivered_at: Option<DateTime<Utc>>,
    pub acknowledged_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BroadcastTarget {
    All,
    Specific,
}

// ─── IPC Request / Response types ───────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub name: String,
    pub duration_minutes: u32,
    pub questions: Vec<QuestionInput>,
    pub allowed_urls: Vec<String>,
    pub options: SessionOptions,
}

#[derive(Debug, Deserialize)]
pub struct QuestionInput {
    pub title: String,
    pub description: String,
    pub input_data: Option<String>,
    pub expected_output: Option<String>,
    #[serde(default)]
    pub visible_testcases: Vec<TestCase>,
    #[serde(default)]
    pub hidden_testcases: Vec<TestCase>,
    pub time_limit_ms: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
    pub code: String,
    pub server_addr: String,
}

#[derive(Debug, Serialize)]
pub struct JoinSessionResponse {
    pub session_id: String,
    pub name: String,
    pub duration_minutes: u32,
    pub questions: Vec<SessionQuestion>,
    pub allowed_urls: Vec<String>,
    pub options: SessionOptions,
    pub remaining_seconds: u64,
}

#[derive(Debug, Deserialize)]
pub struct SubmitCodeRequest {
    pub session_id: String,
    pub student_id: String,
    pub filename: String,
    pub content: String,
    pub lang: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HeartbeatRequest {
    pub session_id: String,
    pub student_id: String,
}

#[derive(Debug, Deserialize)]
pub struct BroadcastRequest {
    pub session_id: String,
    pub content: String,
    pub target_type: BroadcastTarget,
    pub target_ids: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct SessionStatusResponse {
    pub session: Session,
    pub participants: Vec<Participant>,
    pub submission_count: usize,
    pub violation_count: usize,
}
