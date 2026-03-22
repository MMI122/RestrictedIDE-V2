use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::oneshot;
use tower_http::cors::CorsLayer;

use super::db::SessionDb;
use super::models::*;

/// Shared state for the axum server — holds the database.
pub type ServerState = Arc<SessionDb>;

/// Handle to the running LAN server. Dropping it does NOT stop the server;
/// call `stop()` explicitly.
pub struct LanServer {
    pub addr: SocketAddr,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl LanServer {
    /// Start the embedded HTTP server on `0.0.0.0:{port}`.
    /// Returns immediately with a handle; the server runs on the tokio runtime.
    pub async fn start(db: Arc<SessionDb>, port: u16) -> Result<Self, String> {
        let state: ServerState = db;

        let app = Router::new()
            // Session endpoints
            .route("/api/session/{code}/join", post(handle_join))
            .route("/api/session/{id}/status", get(handle_status))
            .route("/api/session/{id}/start", post(handle_start))
            .route("/api/session/{id}/end", post(handle_end))
            .route("/api/session/{id}/submit", post(handle_submit))
            .route("/api/session/{id}/heartbeat", post(handle_heartbeat))
            .route("/api/session/{id}/participants", get(handle_participants))
            .route("/api/session/{id}/submissions", get(handle_submissions))
            .route("/api/session/{id}/violations", get(handle_violations))
            .route("/api/session/{id}/broadcasts/{student_id}", get(handle_student_broadcasts))
            .route("/api/session/{id}/questions", get(handle_questions))
            .route("/api/session/{id}/broadcast", post(handle_broadcast))
            .route("/api/broadcast/{broadcast_id}/delivered", post(handle_broadcast_delivered))
            .route("/api/broadcast/{broadcast_id}/ack", post(handle_broadcast_ack))
            .route("/api/session/{id}/kick", post(handle_kick))
            .route("/api/health", get(handle_health))
            .layer(CorsLayer::permissive())
            .with_state(state);

        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| format!("Failed to bind port {}: {}", port, e))?;
        let actual_addr = listener
            .local_addr()
            .map_err(|e| format!("Failed to get local addr: {}", e))?;

        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .ok();
        });

        log::info!(
            "[LAN Server] Started on http://{}",
            actual_addr
        );

        Ok(LanServer {
            addr: actual_addr,
            shutdown_tx: Some(shutdown_tx),
        })
    }

    /// Gracefully shut down the server.
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
            log::info!("[LAN Server] Shutdown signal sent");
        }
    }

    /// Get the server's local IP address (useful for displaying to students).
    pub fn local_ip() -> Option<String> {
        // Prefer RFC1918 private IPv4 addresses (e.g. 192.168.x.x, 10.x.x.x)
        // and avoid link-local 169.254.x.x addresses that are usually not
        // reachable from other devices on the LAN.
        let addrs = local_ip_address::list_afinet_netifas();
        if let Ok(addrs) = addrs {
            // Pass 1: best candidate = private, non-loopback, non-link-local IPv4.
            for (_, ip) in addrs {
                if let std::net::IpAddr::V4(v4) = ip {
                    if v4.is_private() && !v4.is_loopback() && !v4.is_link_local() {
                        return Some(v4.to_string());
                    }
                }
            }
        }

        // Pass 2: fallback to any non-loopback, non-link-local IPv4.
        let addrs = local_ip_address::list_afinet_netifas();
        if let Ok(addrs) = addrs {
            for (_, ip) in addrs {
                if let std::net::IpAddr::V4(v4) = ip {
                    if !v4.is_loopback() && !v4.is_link_local() {
                        return Some(v4.to_string());
                    }
                }
            }
        }

        Some("127.0.0.1".to_string())
    }
}

impl Drop for LanServer {
    fn drop(&mut self) {
        self.stop();
    }
}

// ─── API response wrapper ───────────────────────────────────────────────────

#[derive(Serialize)]
struct ApiResponse<T: Serialize> {
    ok: bool,
    data: Option<T>,
    error: Option<String>,
}

fn ok_json<T: Serialize>(data: T) -> impl IntoResponse {
    Json(ApiResponse {
        ok: true,
        data: Some(data),
        error: None,
    })
}

fn err_json(status: StatusCode, msg: &str) -> impl IntoResponse {
    (
        status,
        Json(ApiResponse::<()> {
            ok: false,
            data: None,
            error: Some(msg.to_string()),
        }),
    )
}

// ─── Route handlers ─────────────────────────────────────────────────────────

async fn handle_health() -> impl IntoResponse {
    ok_json(serde_json::json!({ "status": "ok" }))
}

// POST /api/session/:code/join
#[derive(Deserialize)]
struct JoinBody {
    student_id: String,
    display_name: Option<String>,
}

async fn handle_join(
    State(db): State<ServerState>,
    Path(code): Path<String>,
    Json(body): Json<JoinBody>,
) -> impl IntoResponse {
    let session = match db.get_session_by_code(&code) {
        Ok(Some(s)) => s,
        Ok(None) => return err_json(StatusCode::NOT_FOUND, "Invalid session code").into_response(),
        Err(e) => return err_json(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    };

    if session.status == SessionStatus::Ended {
        return err_json(StatusCode::GONE, "Session has ended").into_response();
    }

    // Add participant (or re-join if already exists)
    match db.get_participant(&session.id, &body.student_id) {
        Ok(Some(p)) => {
            if p.state == ParticipantState::Kicked {
                return err_json(StatusCode::FORBIDDEN, "You have been removed from this session").into_response();
            }
            // Re-join: update heartbeat
            let _ = db.update_heartbeat(&session.id, &body.student_id);
        }
        Ok(None) => {
            if let Err(e) = db.add_participant(&session.id, &body.student_id, body.display_name.as_deref()) {
                return err_json(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response();
            }
        }
        Err(e) => return err_json(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }

    let questions = db.get_questions(&session.id).unwrap_or_default();

    let remaining = if let Some(ends_at) = session.ends_at {
        let diff = ends_at - chrono::Utc::now();
        diff.num_seconds().max(0) as u64
    } else {
        (session.duration_minutes as u64) * 60
    };

    ok_json(JoinSessionResponse {
        session_id: session.id,
        name: session.name,
        duration_minutes: session.duration_minutes,
        questions,
        allowed_urls: session.allowed_urls,
        options: session.options,
        remaining_seconds: remaining,
    })
    .into_response()
}

// GET /api/session/:id/status
async fn handle_status(
    State(db): State<ServerState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let session = match db.get_session_by_id(&id) {
        Ok(Some(s)) => s,
        Ok(None) => return err_json(StatusCode::NOT_FOUND, "Session not found").into_response(),
        Err(e) => return err_json(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    };
    let participants = db.get_participants(&id).unwrap_or_default();
    let sub_count = db.count_submissions(&id).unwrap_or(0);
    let vio_count = db.count_violations(&id).unwrap_or(0);

    ok_json(SessionStatusResponse {
        session,
        participants,
        submission_count: sub_count,
        violation_count: vio_count,
    })
    .into_response()
}

// POST /api/session/:id/start
async fn handle_start(
    State(db): State<ServerState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match db.start_session(&id) {
        Ok(_) => ok_json(serde_json::json!({ "started": true })).into_response(),
        Err(e) => err_json(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

// POST /api/session/:id/end
async fn handle_end(
    State(db): State<ServerState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match db.end_session(&id) {
        Ok(_) => ok_json(serde_json::json!({ "ended": true })).into_response(),
        Err(e) => err_json(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

// POST /api/session/:id/submit
async fn handle_submit(
    State(db): State<ServerState>,
    Path(id): Path<String>,
    Json(body): Json<SubmitBody>,
) -> impl IntoResponse {
    match db.add_submission(&id, &body.student_id, &body.filename, &body.content, body.lang.as_deref(), true) {
        Ok(sub) => ok_json(sub).into_response(),
        Err(e) => err_json(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct SubmitBody {
    student_id: String,
    filename: String,
    content: String,
    lang: Option<String>,
}

// POST /api/session/:id/heartbeat
async fn handle_heartbeat(
    State(db): State<ServerState>,
    Path(id): Path<String>,
    Json(body): Json<HeartbeatBody>,
) -> impl IntoResponse {
    if let Ok(Some(p)) = db.get_participant(&id, &body.student_id) {
        if p.state == ParticipantState::Kicked {
            return err_json(StatusCode::FORBIDDEN, "You have been removed from this session")
                .into_response();
        }
    }

    match db.update_heartbeat(&id, &body.student_id) {
        Ok(_) => ok_json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => err_json(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct HeartbeatBody {
    student_id: String,
}

// GET /api/session/:id/participants
async fn handle_participants(
    State(db): State<ServerState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match db.get_participants(&id) {
        Ok(ps) => ok_json(ps).into_response(),
        Err(e) => err_json(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

// GET /api/session/:id/submissions
async fn handle_submissions(
    State(db): State<ServerState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match db.get_submissions(&id) {
        Ok(subs) => ok_json(subs).into_response(),
        Err(e) => err_json(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

// GET /api/session/:id/violations
async fn handle_violations(
    State(db): State<ServerState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match db.get_violations(&id) {
        Ok(vs) => ok_json(vs).into_response(),
        Err(e) => err_json(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

// GET /api/session/:id/broadcasts/:student_id
async fn handle_student_broadcasts(
    State(db): State<ServerState>,
    Path((id, student_id)): Path<(String, String)>,
) -> impl IntoResponse {
    match db.get_student_broadcasts(&id, &student_id) {
        Ok(rows) => ok_json(rows).into_response(),
        Err(e) => err_json(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

// GET /api/session/:id/questions
async fn handle_questions(
    State(db): State<ServerState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match db.get_questions(&id) {
        Ok(qs) => ok_json(qs).into_response(),
        Err(e) => err_json(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

// POST /api/session/:id/broadcast
#[derive(Deserialize)]
struct BroadcastBody {
    sender_id: String,
    content: String,
    target_type: Option<String>,
    target_ids: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct BroadcastReceiptBody {
    student_id: String,
}

async fn handle_broadcast(
    State(db): State<ServerState>,
    Path(id): Path<String>,
    Json(body): Json<BroadcastBody>,
) -> impl IntoResponse {
    let target = body.target_type.as_deref().unwrap_or("all");
    match db.add_broadcast(&id, &body.sender_id, &body.content, target, body.target_ids.as_deref()) {
        Ok(b) => ok_json(b).into_response(),
        Err(e) => err_json(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

async fn handle_broadcast_delivered(
    State(db): State<ServerState>,
    Path(broadcast_id): Path<String>,
    Json(body): Json<BroadcastReceiptBody>,
) -> impl IntoResponse {
    match db.mark_broadcast_delivered(&broadcast_id, &body.student_id) {
        Ok(_) => ok_json(serde_json::json!({ "delivered": true })).into_response(),
        Err(e) => err_json(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

async fn handle_broadcast_ack(
    State(db): State<ServerState>,
    Path(broadcast_id): Path<String>,
    Json(body): Json<BroadcastReceiptBody>,
) -> impl IntoResponse {
    match db.acknowledge_broadcast(&broadcast_id, &body.student_id) {
        Ok(_) => ok_json(serde_json::json!({ "acknowledged": true })).into_response(),
        Err(e) => err_json(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

// POST /api/session/:id/kick
#[derive(Deserialize)]
struct KickBody {
    student_id: String,
}

async fn handle_kick(
    State(db): State<ServerState>,
    Path(id): Path<String>,
    Json(body): Json<KickBody>,
) -> impl IntoResponse {
    match db.update_participant_state(&id, &body.student_id, "kicked") {
        Ok(_) => ok_json(serde_json::json!({ "kicked": true })).into_response(),
        Err(e) => err_json(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}
