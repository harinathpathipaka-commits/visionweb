//! WebSocket server for streaming events and receiving human commands.
//!
//! Agents connect at `GET /ws` and receive real-time JSON events:
//! goal progress updates, eye reports, alerts, and session recovery signals.
//!
//! During error recovery, humans send commands (click, type, resume)
//! over the same WebSocket. The server dispatches them directly to the
//! session's CDP backend via `SessionManager`.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::sync::broadcast;
use uuid::Uuid;

use ans_core::session::SessionStatus;
use ans_core::backend::ImageFormat;
use ans_goal::GoalManager;
use ans_ipc::session::SessionManager;

/// WebSocket server state.
#[derive(Clone)]
pub struct WebSocketServer {
    /// Local event bus for forwarding to WS clients (bidirectional).
    event_tx: broadcast::Sender<String>,
    /// Session manager for dispatching human actions to CDP backends.
    sessions: Option<SessionManager>,
    /// Goal manager for unblocking goals on human resume.
    goals: Option<GoalManager>,
}

impl WebSocketServer {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self { event_tx, sessions: None, goals: None }
    }

    /// Attach session and goal managers (called after Gateway::init).
    pub fn with_managers(mut self, sessions: SessionManager, goals: GoalManager) -> Self {
        self.sessions = Some(sessions.clone());
        self.goals = Some(goals);
        self
    }

    /// Access the session manager for bridge tasks (e.g., screenshot capture).
    pub fn sessions(&self) -> Option<&SessionManager> {
        self.sessions.as_ref()
    }

    /// Subscribe to internal events.
    pub fn event_sender(&self) -> broadcast::Sender<String> {
        self.event_tx.clone()
    }

    /// Push an event to all connected WebSocket clients.
    pub fn push_event(&self, event: &str) {
        let _ = self.event_tx.send(event.to_string());
    }

    /// Handle a WebSocket upgrade request.
    pub async fn handle_upgrade(&self, ws: WebSocketUpgrade) -> impl IntoResponse {
        let event_rx = self.event_tx.subscribe();
        let sessions = self.sessions.clone();
        let goals = self.goals.clone();
        let event_tx = self.event_tx.clone();
        ws.on_upgrade(move |socket| handle_socket(socket, event_rx, event_tx, sessions, goals))
    }
}

/// Run the WebSocket event loop.
async fn handle_socket(
    socket: WebSocket,
    server_rx: broadcast::Receiver<String>,
    event_tx: broadcast::Sender<String>,
    sessions: Option<SessionManager>,
    goals: Option<GoalManager>,
) {
    let (mut sender, mut receiver) = socket.split();

    // Welcome message
    let welcome = json!({
        "type": "connected",
        "message": "Connected to Agent Nervous System gateway",
        "version": env!("CARGO_PKG_VERSION")
    });
    let _ = sender.send(Message::Text(welcome.to_string())).await;

    // Forward server events → this WS client
    let mut event_rx = server_rx.resubscribe();
    let mut send_clone = sender;
    let send_handle = tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            if send_clone.send(Message::Text(event)).await.is_err() {
                break;
            }
        }
    });

    // Process incoming messages (human commands during error recovery)
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Ping(_) => {}
            Message::Close(_) => break,
            Message::Text(text) => {
                handle_incoming_command(
                    &text, &sessions, &goals, &event_tx,
                ).await;
            }
            _ => {}
        }
    }

    send_handle.abort();
}

/// Parse and dispatch an incoming WebSocket command from the dashboard.
async fn handle_incoming_command(
    text: &str,
    sessions: &Option<SessionManager>,
    goals: &Option<GoalManager>,
    event_tx: &broadcast::Sender<String>,
) {
    let cmd: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => return,
    };

    let cmd_type = cmd.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let session_id = cmd.get("session_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok());

    let sm = match sessions {
        Some(s) => s,
        None => return,
    };

    match (cmd_type, session_id) {
        // ── Human click at page coordinates ──────────────────────
        ("human_click", Some(sid)) => {
            let x = cmd.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = cmd.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            match sm.click_at(sid, x, y).await {
                Ok(result) => push_ws_event(event_tx, "action_result", json!({
                    "session_id": sid.to_string(), "success": true, "result": result
                })),
                Err(e) => push_ws_event(event_tx, "action_result", json!({
                    "session_id": sid.to_string(), "success": false, "result": e.to_string()
                })),
            }
            push_live_screenshot(event_tx, sm, sid).await;
        }
        // ── Type text into focused element ───────────────────────
        ("human_type", Some(sid)) => {
            let text_val = cmd.get("text").and_then(|v| v.as_str()).unwrap_or("");
            match sm.type_to_page(sid, text_val).await {
                Ok(result) => push_ws_event(event_tx, "action_result", json!({
                    "session_id": sid.to_string(), "success": true, "result": result
                })),
                Err(e) => push_ws_event(event_tx, "action_result", json!({
                    "session_id": sid.to_string(), "success": false, "result": e.to_string()
                })),
            }
            push_live_screenshot(event_tx, sm, sid).await;
        }
        // ── Key press (Enter, Tab, Escape, Backspace) ────────────
        ("human_key", Some(sid)) => {
            let key = cmd.get("key").and_then(|v| v.as_str()).unwrap_or("");
            match sm.key_press(sid, key).await {
                Ok(result) => push_ws_event(event_tx, "action_result", json!({
                    "session_id": sid.to_string(), "success": true, "result": result
                })),
                Err(e) => push_ws_event(event_tx, "action_result", json!({
                    "session_id": sid.to_string(), "success": false, "result": e.to_string()
                })),
            }
            push_live_screenshot(event_tx, sm, sid).await;
        }
        // ── Scroll page ─────────────────────────────────────────
        ("human_scroll", Some(sid)) => {
            let pixels = cmd.get("pixels").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            match sm.scroll_by(sid, pixels).await {
                Ok(result) => push_ws_event(event_tx, "action_result", json!({
                    "session_id": sid.to_string(), "success": true, "result": result
                })),
                Err(e) => push_ws_event(event_tx, "action_result", json!({
                    "session_id": sid.to_string(), "success": false, "result": e.to_string()
                })),
            }
            push_live_screenshot(event_tx, sm, sid).await;
        }
        // ── Refresh screenshot ───────────────────────────────────
        ("refresh_screenshot", Some(sid)) => {
            push_live_screenshot(event_tx, sm, sid).await;
        }
        // ── Resume agent ────────────────────────────────────────
        ("resume_agent", Some(sid)) => {
            let goal_id = cmd.get("goal_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok());
            if let (Some(ref goals_mgr), Some(gid)) = (goals, goal_id) {
                let _ = goals_mgr.update_progress(gid, 0.0, ans_core::goal::GoalStatus::Active, None);
            }
            sm.set_status(sid, SessionStatus::Idle).await;
            if let Some(gid) = goal_id {
                push_ws_event(event_tx, "agent_resumed", json!({
                    "session_id": sid.to_string(),
                    "goal_id": gid.to_string()
                }));
            }
        }
        // ── Cancel goal ──────────────────────────────────────────
        ("cancel_goal", Some(sid)) => {
            let goal_id = cmd.get("goal_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok());
            if let (Some(ref goals_mgr), Some(gid)) = (goals, goal_id) {
                let _ = goals_mgr.update_progress(gid, 0.0, ans_core::goal::GoalStatus::Failed, None);
            }
            sm.set_status(sid, SessionStatus::Failed).await;
        }
        _ => {
            tracing::debug!(%cmd_type, "Unknown WS command ignored");
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn push_ws_event(tx: &broadcast::Sender<String>, event_type: &str, data: serde_json::Value) {
    let mut evt = data;
    if let Some(obj) = evt.as_object_mut() {
        obj.insert("type".into(), serde_json::Value::String(event_type.into()));
    }
    let _ = tx.send(evt.to_string());
}

async fn push_live_screenshot(
    tx: &broadcast::Sender<String>,
    sm: &SessionManager,
    session_id: Uuid,
) {
    match sm.capture_screenshot(session_id, ImageFormat::Png, false).await {
        Ok(bytes) => {
            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
            push_ws_event(tx, "live_screenshot", json!({
                "session_id": session_id.to_string(),
                "data_base64": b64
            }));
        }
        Err(e) => {
            tracing::warn!(%session_id, %e, "Failed to capture live screenshot");
        }
    }
}
