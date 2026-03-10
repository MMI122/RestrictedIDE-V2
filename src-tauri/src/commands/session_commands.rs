use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

use crate::session::db::SessionDb;
use crate::session::lan_server::LanServer;
use crate::session::models::*;
use crate::session::transport::{LanTransport, SessionTransport, TransportError};

// ─── Shared session state (stored in Tauri managed state) ───────────────────

pub struct SessionState {
    pub transport: std::sync::Mutex<Option<Box<dyn SessionTransport>>>,
    pub lan_server: tokio::sync::Mutex<Option<LanServer>>,
    pub db: Arc<SessionDb>,
    pub current_session_id: std::sync::Mutex<Option<String>>,
    pub role: std::sync::Mutex<SessionRole>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionRole {
    None,
    Admin,
    Student,
}

impl Default for SessionRole {
    fn default() -> Self {
        Self::None
    }
}

impl SessionState {
    pub fn new(db: Arc<SessionDb>) -> Self {
        let transport = LanTransport::new(db.clone());
        Self {
            transport: std::sync::Mutex::new(Some(Box::new(transport))),
            lan_server: tokio::sync::Mutex::new(None),
            db,
            current_session_id: std::sync::Mutex::new(None),
            role: std::sync::Mutex::new(SessionRole::None),
        }
    }

    fn get_transport(&self) -> Result<Box<dyn SessionTransport + '_>, String> {
        // We need to return a reference to the inner transport.
        // Since the trait is behind a Mutex<Option<Box<dyn>>>, the simplest
        // approach is to create a fresh LanTransport each call (cheap).
        Ok(Box::new(LanTransport::new(self.db.clone())))
    }
}

fn transport_err(e: TransportError) -> String {
    e.to_string()
}

// ─── Tauri IPC Commands ─────────────────────────────────────────────────────

const DEFAULT_PORT: u16 = 9876;

#[tauri::command]
pub async fn create_session_cmd(
    session_state: State<'_, SessionState>,
    name: String,
    duration_minutes: u32,
    questions: Vec<QuestionInput>,
    allowed_urls: Vec<String>,
    options: SessionOptions,
) -> Result<CreateSessionResponse, String> {
    let transport = session_state.get_transport()?;

    let req = CreateSessionRequest {
        name,
        duration_minutes,
        questions,
        allowed_urls,
        options,
    };

    let mut resp = transport
        .create_session(req, "admin")
        .map_err(transport_err)?;

    // Start the LAN server if not already running
    {
        let mut server = session_state.lan_server.lock().await;
        if server.is_none() {
            let srv = LanServer::start(session_state.db.clone(), DEFAULT_PORT)
                .await
                .map_err(|e| format!("Failed to start LAN server: {}", e))?;
            let ip = LanServer::local_ip().unwrap_or_else(|| "127.0.0.1".into());
            resp.server_addr = format!("{}:{}", ip, srv.addr.port());
            *server = Some(srv);
        } else if let Some(srv) = server.as_ref() {
            let ip = LanServer::local_ip().unwrap_or_else(|| "127.0.0.1".into());
            resp.server_addr = format!("{}:{}", ip, srv.addr.port());
        }
    }

    // Set role and current session
    *session_state.role.lock().unwrap() = SessionRole::Admin;
    *session_state.current_session_id.lock().unwrap() = Some(resp.session_id.clone());

    log::info!(
        "[Session] Created '{}' code={} addr={}",
        resp.session_id, resp.code, resp.server_addr
    );

    Ok(resp)
}

#[tauri::command]
pub async fn start_session_cmd(
    session_state: State<'_, SessionState>,
    session_id: String,
) -> Result<(), String> {
    let transport = session_state.get_transport()?;
    transport.start_session(&session_id).map_err(transport_err)?;
    log::info!("[Session] Started {}", session_id);
    Ok(())
}

#[tauri::command]
pub async fn end_session_cmd(
    session_state: State<'_, SessionState>,
    session_id: String,
) -> Result<(), String> {
    let transport = session_state.get_transport()?;
    transport.end_session(&session_id).map_err(transport_err)?;
    log::info!("[Session] Ended {}", session_id);
    Ok(())
}

#[tauri::command]
pub async fn delete_session_cmd(
    session_state: State<'_, SessionState>,
    session_id: String,
) -> Result<(), String> {
    let transport = session_state.get_transport()?;
    transport.delete_session(&session_id).map_err(transport_err)?;
    log::info!("[Session] Deleted {}", session_id);
    Ok(())
}

#[tauri::command]
pub async fn list_sessions_cmd(
    session_state: State<'_, SessionState>,
) -> Result<Vec<Session>, String> {
    let transport = session_state.get_transport()?;
    transport.list_sessions().map_err(transport_err)
}

#[tauri::command]
pub async fn join_session_cmd(
    session_state: State<'_, SessionState>,
    code: String,
    student_id: String,
) -> Result<JoinSessionResponse, String> {
    let transport = session_state.get_transport()?;
    let resp = transport
        .join_session(&code, &student_id)
        .map_err(transport_err)?;

    *session_state.role.lock().unwrap() = SessionRole::Student;
    *session_state.current_session_id.lock().unwrap() = Some(resp.session_id.clone());

    log::info!(
        "[Session] Student '{}' joined session '{}' ({})",
        student_id, resp.session_id, resp.name
    );

    Ok(resp)
}

#[tauri::command]
pub async fn submit_code_cmd(
    session_state: State<'_, SessionState>,
    session_id: String,
    student_id: String,
    filename: String,
    content: String,
    lang: Option<String>,
) -> Result<Submission, String> {
    let transport = session_state.get_transport()?;
    let req = SubmitCodeRequest {
        session_id: session_id.clone(),
        student_id: student_id.clone(),
        filename,
        content,
        lang,
    };
    let sub = transport.submit_code(req).map_err(transport_err)?;
    log::info!(
        "[Session] Student '{}' submitted to '{}'",
        student_id, session_id
    );
    Ok(sub)
}

#[tauri::command]
pub async fn heartbeat_cmd(
    session_state: State<'_, SessionState>,
    session_id: String,
    student_id: String,
) -> Result<(), String> {
    let transport = session_state.get_transport()?;
    transport
        .heartbeat(HeartbeatRequest {
            session_id,
            student_id,
        })
        .map_err(transport_err)
}

#[tauri::command]
pub async fn get_session_status_cmd(
    session_state: State<'_, SessionState>,
    session_id: String,
) -> Result<SessionStatusResponse, String> {
    let transport = session_state.get_transport()?;
    transport
        .get_session_status(&session_id)
        .map_err(transport_err)
}

#[tauri::command]
pub async fn get_session_participants_cmd(
    session_state: State<'_, SessionState>,
    session_id: String,
) -> Result<Vec<Participant>, String> {
    let transport = session_state.get_transport()?;
    transport
        .get_participants(&session_id)
        .map_err(transport_err)
}

#[tauri::command]
pub async fn get_session_submissions_cmd(
    session_state: State<'_, SessionState>,
    session_id: String,
) -> Result<Vec<Submission>, String> {
    let transport = session_state.get_transport()?;
    transport
        .get_submissions(&session_id)
        .map_err(transport_err)
}

#[tauri::command]
pub async fn get_session_violations_cmd(
    session_state: State<'_, SessionState>,
    session_id: String,
) -> Result<Vec<Violation>, String> {
    let transport = session_state.get_transport()?;
    transport
        .get_violations(&session_id)
        .map_err(transport_err)
}

#[tauri::command]
pub async fn broadcast_message_cmd(
    session_state: State<'_, SessionState>,
    session_id: String,
    content: String,
    target_type: String,
    target_ids: Option<Vec<String>>,
) -> Result<Broadcast, String> {
    let transport = session_state.get_transport()?;
    let bt = if target_type == "specific" {
        BroadcastTarget::Specific
    } else {
        BroadcastTarget::All
    };
    let req = BroadcastRequest {
        session_id,
        content,
        target_type: bt,
        target_ids,
    };
    transport.broadcast(req, "admin").map_err(transport_err)
}

#[tauri::command]
pub async fn kick_participant_cmd(
    session_state: State<'_, SessionState>,
    session_id: String,
    student_id: String,
) -> Result<(), String> {
    let transport = session_state.get_transport()?;
    transport
        .kick_participant(&session_id, &student_id)
        .map_err(transport_err)
}

#[tauri::command]
pub async fn stop_lan_server_cmd(
    session_state: State<'_, SessionState>,
) -> Result<(), String> {
    let mut server = session_state.lan_server.lock().await;
    if let Some(mut srv) = server.take() {
        srv.stop();
        log::info!("[LAN Server] Stopped");
    }
    Ok(())
}

#[tauri::command]
pub async fn get_current_role_cmd(
    session_state: State<'_, SessionState>,
) -> Result<SessionRole, String> {
    Ok(session_state.role.lock().unwrap().clone())
}
