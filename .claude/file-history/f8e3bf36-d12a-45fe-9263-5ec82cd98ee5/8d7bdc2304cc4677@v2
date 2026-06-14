//! Layer 1 External API Gateway.
//!
//! How external AI agents connect to the Agent Nervous System.
//! Three protocols, one gateway:
//!
//! - **MCP server** (Model Context Protocol): JSON-RPC tool provider
//!   for Claude, `ChatGPT`, and other MCP-compatible agents.
//! - **REST API**: Simple HTTP endpoints for non-MCP agents.
//! - **WebSocket**: Streaming events for real-time agent feedback.
//!
//! All connections are authenticated (API key), rate-limited, and
//! budget-tracked before any internal gRPC call is made.

pub mod auth;
pub mod client;
pub mod mcp;
pub mod metrics;
pub mod rest;
pub mod ws;

use std::net::SocketAddr;
use axum::routing::{get, post};
use axum::Router;

use std::sync::Arc;

use ans_goal::GoalStateStore;
use auth::SharedKeyRegistry;
use client::InternalClient;
use mcp::McpServer;
use metrics::Metrics;
use rest::ApiRouter;
use ws::WebSocketServer;

/// Shared gateway state, created once at startup.
#[derive(Clone)]
pub struct Gateway {
    key_registry: SharedKeyRegistry,
    pub metrics: Arc<Metrics>,
    mcp: McpServer,
    rest: ApiRouter,
    ws: WebSocketServer,
}

impl Gateway {
    /// Initialize the gateway: load API keys, connect to the daemon.
    ///
    /// If `goal_store` is provided, a background task bridges goal state
    /// notifications to all connected WebSocket clients.
    pub async fn init(
        grpc_port: u16,
        goal_store: Option<GoalStateStore>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let key_registry = Arc::new(auth::KeyRegistry::load(None)?);
        let internal = InternalClient::connect(grpc_port).await?;
        let metrics = Arc::new(Metrics::new());
        let mcp = McpServer::new(internal.clone());
        let rest = ApiRouter::new(internal.clone());
        let ws = WebSocketServer::new(internal);

        // Bridge goal state changes to WebSocket clients
        if let Some(store) = &goal_store {
            let ws_clone = ws.clone();
            let mut rx = store.subscribe();
            tokio::spawn(async move {
                while let Ok(notification) = rx.recv().await {
                    let json = serde_json::to_string(&serde_json::json!({
                        "type": "goal_update",
                        "goal_id": notification.goal_id.to_string(),
                        "progress": notification.progress,
                        "status": format!("{:?}", notification.status),
                        "message": notification.message,
                    }))
                    .unwrap_or_default();
                    ws_clone.push_event(&json);
                }
            });
            tracing::info!("WebSocket bridge: forwarding goal updates to clients");
        }

        Ok(Self {
            key_registry,
            metrics,
            mcp,
            rest,
            ws,
        })
    }

    /// Build the axum router.
    ///
    /// **One flat Router** — no `nest()`, no `merge()`.
    /// axum 0.7 `nest()` drops `{param}` captures and `merge()` with
    /// `route_layer` silently discards parameterized routes.
    /// A single flat Router with path-aware auth is the only reliable pattern.
    pub fn router(&self) -> Router {
        let rest = self.rest.clone();
        let mcp = self.mcp.clone();
        let ws = self.ws.clone();
        let registry = Arc::clone(&self.key_registry);
        let metrics_arc = Arc::clone(&self.metrics);

        // Path-aware auth middleware: skips public routes, enforces API key on
        // everything else.
        let auth_mw = axum::middleware::from_fn({
            let registry = Arc::clone(&registry);
            move |request: axum::extract::Request, next: axum::middleware::Next| {
                let registry = Arc::clone(&registry);
                async move {
                    let path = request.uri().path().to_owned();

                    // Public routes — no API key required.
                    if path == "/api/v1/health"
                        || path == "/api/v1/metrics"
                        || path == "/mcp"
                        || path == "/ws"
                        || path == "/"
                    {
                        return next.run(request).await;
                    }

                    let api_key = auth::extract_api_key(request.headers(), None);
                    let valid = api_key
                        .as_ref()
                        .and_then(|k| registry.validate(k))
                        .is_some();
                    if valid {
                        next.run(request).await
                    } else {
                        let body = if api_key.is_some() {
                            r#"{"error":"invalid API key"}"#
                        } else {
                            r#"{"error":"missing X-API-Key header"}"#
                        };
                        axum::response::Response::builder()
                            .status(axum::http::StatusCode::UNAUTHORIZED)
                            .body(axum::body::Body::from(body))
                            .expect("building a 401 response is infallible")
                    }
                }
            }
        });

        // Single flat Router — every route registered directly.
        // No nest(), no merge() — both break {param} captures in axum 0.7.
        Router::new()
            // ── Public routes ──────────────────────────────────────
            .route(
                "/",
                get(|| async {
                    axum::response::Html(include_str!("../../../static/dashboard.html"))
                }),
            )
            .route(
                "/mcp",
                post(
                    move |body: axum::extract::Json<serde_json::Value>| async move {
                        let response = mcp.handle_request(body.0).await;
                        axum::Json(response)
                    },
                ),
            )
            .route(
                "/ws",
                get(
                    move |ws_upgrade: axum::extract::ws::WebSocketUpgrade| async move {
                        ws.handle_upgrade(ws_upgrade).await
                    },
                ),
            )
            .route(
                "/api/v1/health",
                get({
                    let rest = rest.clone();
                    move || {
                        let rest = rest.clone();
                        async move { rest.health().await }
                    }
                }),
            )
            .route(
                "/api/v1/metrics",
                get(move || {
                    let m = Arc::clone(&metrics_arc);
                    async move { m.render() }
                }),
            )
            // ── Session routes ─────────────────────────────────────
            .route(
                "/api/v1/sessions",
                post({
                    let rest = rest.clone();
                    move |axum::extract::Json(body): axum::extract::Json<serde_json::Value>| async move {
                        rest.create_session(body).await
                    }
                }),
            )
            .route(
                "/api/v1/sessions/{id}/navigate",
                post({
                    let rest = rest.clone();
                    move |axum::extract::Path(id): axum::extract::Path<String>,
                          axum::extract::Json(body): axum::extract::Json<serde_json::Value>| async move {
                        rest.navigate(&id, body).await
                    }
                }),
            )
            .route(
                "/api/v1/sessions/{id}/execute",
                post({
                    let rest = rest.clone();
                    move |axum::extract::Path(id): axum::extract::Path<String>,
                          axum::extract::Json(body): axum::extract::Json<serde_json::Value>| async move {
                        rest.execute_action(&id, body).await
                    }
                }),
            )
            .route(
                "/api/v1/sessions/{id}/screenshot",
                post({
                    let rest = rest.clone();
                    move |axum::extract::Path(id): axum::extract::Path<String>,
                          axum::extract::Json(body): axum::extract::Json<serde_json::Value>| async move {
                        rest.screenshot(&id, body).await
                    }
                }),
            )
            .route(
                "/api/v1/sessions/{id}/dom",
                get({
                    let rest = rest.clone();
                    move |axum::extract::Path(id): axum::extract::Path<String>| async move {
                        rest.get_dom(&id).await
                    }
                }),
            )
            // ── Goal routes ────────────────────────────────────────
            .route(
                "/api/v1/goals",
                post({
                    let rest = rest.clone();
                    move |axum::extract::Json(body): axum::extract::Json<serde_json::Value>| async move {
                        rest.create_goal(body).await
                    }
                }),
            )
            .route(
                "/api/v1/goals/{id}",
                get({
                    let rest = rest.clone();
                    move |axum::extract::Path(id): axum::extract::Path<String>| async move {
                        rest.get_goal(&id).await
                    }
                }),
            )
            // Auth middleware applies to everything, but skips public paths.
            .route_layer(auth_mw)
    }

    /// Start the gateway HTTP server on the given address.
    /// Blocks until the server exits.
    pub async fn serve(self, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        let router = self.router();

        tracing::info!("Gateway starting on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, router).await?;

        Ok(())
    }
}
