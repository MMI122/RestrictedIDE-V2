use std::path::PathBuf;
use std::sync::Arc;

use restricted_ide_lib::session::db::SessionDb;
use restricted_ide_lib::session::models::{
    BroadcastRequest, BroadcastTarget, CreateSessionRequest, ParticipantState, QuestionInput,
    SessionOptions, SessionStatus, SubmitCodeRequest,
};
use restricted_ide_lib::session::transport::{LanTransport, SessionTransport, TransportError};
use uuid::Uuid;

fn temp_db_path(test_name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "restricted_ide_{}_{}.db",
        test_name,
        Uuid::new_v4()
    ))
}

fn cleanup_db_files(db_path: &PathBuf) {
    let _ = std::fs::remove_file(db_path);
    let _ = std::fs::remove_file(format!("{}-wal", db_path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", db_path.display()));
}

fn make_transport(test_name: &str) -> (LanTransport, Arc<SessionDb>, PathBuf) {
    let db_path = temp_db_path(test_name);
    let db = Arc::new(SessionDb::open(&db_path).expect("open session db"));
    (LanTransport::new(db.clone()), db, db_path)
}

fn sample_create_session_request() -> CreateSessionRequest {
    CreateSessionRequest {
        name: "Midterm".to_string(),
        duration_minutes: 60,
        questions: vec![QuestionInput {
            title: "Q1".to_string(),
            description: "Print hello".to_string(),
            input_data: Some("".to_string()),
            expected_output: Some("hello".to_string()),
            visible_testcases: vec![],
            hidden_testcases: vec![],
            time_limit_ms: Some(2000),
        }],
        allowed_urls: vec!["https://docs.python.org/*".to_string()],
        options: SessionOptions::default(),
    }
}

#[test]
fn create_start_join_submit_flow_works_end_to_end() {
    let (transport, db, db_path) = make_transport("e2e_flow");

    let created = transport
        .create_session(sample_create_session_request(), "admin")
        .expect("create session");

    let session = db
        .get_session_by_id(&created.session_id)
        .expect("read session")
        .expect("session exists");
    assert_eq!(session.status, SessionStatus::Created);

    transport
        .start_session(&created.session_id)
        .expect("start session");

    let joined = transport
        .join_session("", &created.code, "student-1", Some("Alice"), Some("device-a"))
        .expect("join session");
    assert_eq!(joined.session_id, created.session_id);
    assert_eq!(joined.questions.len(), 1);
    assert!(joined.remaining_seconds > 0);

    transport
        .heartbeat(restricted_ide_lib::session::models::HeartbeatRequest {
            session_id: created.session_id.clone(),
            student_id: "student-1".to_string(),
        })
        .expect("heartbeat");

    transport
        .submit_code(SubmitCodeRequest {
            session_id: created.session_id.clone(),
            student_id: "student-1".to_string(),
            filename: "main.py".to_string(),
            content: "print('hello')".to_string(),
            lang: Some("python".to_string()),
        })
        .expect("submit code");

    let participant = db
        .get_participant(&created.session_id, "student-1")
        .expect("read participant")
        .expect("participant exists");
    assert_eq!(participant.state, ParticipantState::Submitted);

    let status = transport
        .get_session_status(&created.session_id)
        .expect("status");
    assert_eq!(status.submission_count, 1);

    drop(transport);
    drop(db);
    cleanup_db_files(&db_path);
}

#[test]
fn join_is_blocked_when_device_is_locked_by_other_student() {
    let (transport, db, db_path) = make_transport("device_lock");

    let created = transport
        .create_session(sample_create_session_request(), "admin")
        .expect("create session");

    transport
        .join_session("", &created.code, "student-a", Some("Alice"), Some("lab-pc-1"))
        .expect("first join succeeds");

    transport
        .kick_participant(&created.session_id, "student-a")
        .expect("kick first participant");

    let err = transport
        .join_session("", &created.code, "student-b", Some("Bob"), Some("lab-pc-1"))
        .expect_err("join should require admin approval");

    match err {
        TransportError::InvalidState(message) => {
            assert!(message.contains("Administrator approval is required"));
        }
        _ => panic!("unexpected error kind"),
    }

    let pending = db
        .get_participant(&created.session_id, "student-b")
        .expect("read pending participant")
        .expect("pending participant exists");
    assert_eq!(pending.state, ParticipantState::ReentryPending);

    drop(transport);
    drop(db);
    cleanup_db_files(&db_path);
}

#[test]
fn broadcast_all_skips_kicked_participants() {
    let (transport, db, db_path) = make_transport("broadcast_receipts");

    let created = transport
        .create_session(sample_create_session_request(), "admin")
        .expect("create session");

    transport
        .join_session("", &created.code, "student-1", Some("A"), Some("d1"))
        .expect("join student-1");
    transport
        .join_session("", &created.code, "student-2", Some("B"), Some("d2"))
        .expect("join student-2");

    transport
        .kick_participant(&created.session_id, "student-2")
        .expect("kick student-2");

    transport
        .broadcast(
            BroadcastRequest {
                session_id: created.session_id.clone(),
                content: "Read instructions".to_string(),
                target_type: BroadcastTarget::All,
                target_ids: None,
            },
            "admin",
        )
        .expect("send broadcast");

    let s1_msgs = db
        .get_student_broadcasts(&created.session_id, "student-1")
        .expect("student-1 broadcasts");
    let s2_msgs = db
        .get_student_broadcasts(&created.session_id, "student-2")
        .expect("student-2 broadcasts");

    assert_eq!(s1_msgs.len(), 1);
    assert_eq!(s2_msgs.len(), 0);

    drop(transport);
    drop(db);
    cleanup_db_files(&db_path);
}
