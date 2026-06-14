//! CDP WebSocket client implementing [`ans_core::BrowserBackend`].
//!
//! Architecture:
//! ```text
//! CdpBackend
//!   ├── ChromiumProcess (owns child process)
//!   └── CdpConnection
//!         ├── write_tx: mpsc::Sender<String>  (outgoing commands)
//!         ├── pending: Arc<Mutex<HashMap<id, oneshot::Sender>>>
//!         └── reader task (spawned, reads WS messages)
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use ans_core::backend::{BrowserError, ImageFormat, PageState};
use ans_core::distill::{DistillMode, DistilledPage};
use ans_core::session::ActionOutcome;
use ans_stealth::StealthConfig;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_tungstenite::tungstenite::Message;
use tracing;

use crate::command::{self, CdpCommand, CdpError, CdpMessage};
use crate::process::ChromiumProcess;

// ── CdpConnection ────────────────────────────────────────────────────

type PendingMap = Arc<Mutex<HashMap<i64, oneshot::Sender<Result<serde_json::Value, CdpError>>>>>;

struct CdpConnection {
    write_tx: mpsc::Sender<String>,
    pending: PendingMap,
}

impl CdpConnection {
    /// Establish a WebSocket connection and spawn the reader task.
    async fn connect(debug_url: &str) -> Result<Self, BrowserError> {
        let (ws, _) = tokio_tungstenite::connect_async(debug_url)
            .await
            .map_err(|e| BrowserError::ConnectionLost(e.to_string()))?;

        let (mut write, read) = ws.split();
        let (write_tx, mut write_rx) = mpsc::channel::<String>(256);
        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));

        // Writer task: pumps messages from the mpsc channel to the WebSocket
        let writer_pending = pending.clone();
        tokio::spawn(async move {
            while let Some(msg) = write_rx.recv().await {
                if let Err(e) = write.send(Message::Text(msg)).await {
                    let _ = writer_pending.lock().await;
                    tracing::error!(?e, "CDP write failed");
                    break;
                }
            }
        });

        // Reader task: reads WebSocket messages, dispatches responses to pending
        // commands, and forwards events to the broadcast channel
        let reader_pending = pending.clone();
        tokio::spawn(async move {
            let mut read = read;
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        let text_str = text.clone();
                        if let Some(cdp_msg) = command::parse_message(&text_str) {
                            match cdp_msg {
                                CdpMessage::Response(resp) => {
                                    let id = resp.id;
                                    let result = resp.into_result();
                                    let mut map = reader_pending.lock().await;
                                    if let Some(tx) = map.remove(&id) {
                                        let _ = tx.send(result);
                                    }
                                }
                                CdpMessage::Event(event) => {
                                    tracing::trace!(
                                        method = %event.method,
                                        "CDP event"
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(?e, "CDP WebSocket read error");
                        break;
                    }
                    _ => {}
                }
            }
            tracing::debug!("CDP reader task exiting");
        });

        Ok(Self { write_tx, pending })
    }

    /// Send a CDP command and await its response with a timeout.
    async fn send_command(
        &self,
        cmd: CdpCommand,
        timeout: Duration,
    ) -> Result<serde_json::Value, BrowserError> {
        let (tx, rx) = oneshot::channel();
        let id = cmd.id;

        self.pending.lock().await.insert(id, tx);
        let json = cmd.to_json();

        self.write_tx
            .send(json)
            .await
            .map_err(|e| BrowserError::ConnectionLost(e.to_string()))?;

        tokio::time::timeout(timeout, rx)
            .await
            .map_err(|_| BrowserError::NavigationTimeout(timeout))?
            .map_err(|_| BrowserError::ConnectionLost("sender dropped".into()))?
            .map_err(|e| BrowserError::CdpProtocol(e.to_string()))
    }
}

// ── CdpBackend ───────────────────────────────────────────────────────

/// CDP-backed browser implementation.
///
/// Owns the Chromium child process and the WebSocket connection.
pub struct CdpBackend {
    // Held for RAII cleanup: Chromium is killed on Drop.
    _process: ChromiumProcess,
    connection: CdpConnection,
    // Current page state, updated after navigations.
    current_url: Mutex<String>,
    /// Stealth configuration (anti-detection engine).
    /// Stored for use in mouse/keyboard humanization.
    stealth: Option<StealthConfig>,
}

impl CdpBackend {
    /// Launch a new Chromium instance and establish a CDP connection.
    ///
    /// When `stealth` is provided, anti-detection scripts are injected
    /// via `Page.addScriptToEvaluateOnNewDocument` before any page loads.
    pub async fn launch(stealth: Option<StealthConfig>) -> Result<Self, BrowserError> {
        let process = ChromiumProcess::launch(stealth.as_ref())
            .await
            .map_err(|e| BrowserError::Internal(e.into()))?;

        // Connect to the page-level WebSocket URL (not browser-level).
        // Page.*, DOM.*, and Runtime.* commands require a page target connection.
        let page_url = process.page_url().to_string();
        let connection = CdpConnection::connect(&page_url).await?;

        // Enable domains we need
        connection
            .send_command(command::page_enable(), Duration::from_secs(10))
            .await?;
        connection
            .send_command(command::dom_enable(), Duration::from_secs(10))
            .await?;
        connection
            .send_command(command::runtime_enable(), Duration::from_secs(10))
            .await?;

        // Inject anti-detection scripts before any page loads
        if let Some(ref config) = stealth {
            let scripts = config.init_scripts();
            if !scripts.is_empty() {
                tracing::info!(
                    count = scripts.len(),
                    level = ?config.level,
                    "Injecting stealth scripts"
                );
                for script in &scripts {
                    let cmd = command::page_add_script_to_evaluate_on_new_document(script);
                    if let Err(e) = connection
                        .send_command(cmd, Duration::from_secs(10))
                        .await
                    {
                        tracing::warn!(?e, "Failed to inject stealth script");
                    }
                }
            }
        }

        tracing::info!("CDP backend ready");

        Ok(Self {
            _process: process,
            connection,
            current_url: Mutex::new(String::new()),
            stealth,
        })
    }

    /// Wait for the page to settle after an interaction. Polls
    /// `document.readyState` until it is `"complete"` or the timeout
    /// expires.  Keeps the next DOM capture from seeing stale state.
    ///
    /// Early exit: checks readyState once before entering the poll
    /// loop. If the page is already settled (the common case for static
    /// pages and most form interactions), returns in ~50ms instead of
    /// burning the full poll interval.
    async fn wait_for_stability(
        connection: &CdpConnection,
        timeout: Duration,
    ) {
        // Fast path: check readyState once before entering the poll loop.
        // Most pages are already "complete" after a click — we only need
        // the loop for slow SPA transitions and full navigations.
        let cmd = command::runtime_evaluate("document.readyState");
        if let Ok(response) = connection.send_command(cmd, Duration::from_secs(2)).await {
            if let Ok(value) = command::parse_evaluate_result(&response) {
                if value.as_str() == Some("complete") {
                    return; // already settled — no wait needed
                }
            }
        }
        // Slow path: poll until complete or timeout
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let cmd = command::runtime_evaluate("document.readyState");
            match connection.send_command(cmd, Duration::from_secs(2)).await {
                Ok(response) => {
                    if let Ok(value) = command::parse_evaluate_result(&response) {
                        if value.as_str() == Some("complete") {
                            return;
                        }
                    }
                }
                Err(_) => return, // connection closed, give up
            }
            if tokio::time::Instant::now() >= deadline {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Send a command and drop the response (fire-and-forget).
    async fn send_ignore(&self, cmd: CdpCommand) {
        let json = cmd.to_json();
        let _ = self.connection.write_tx.send(json).await;
    }
}

#[async_trait]
impl ans_core::BrowserBackend for CdpBackend {
    async fn navigate(&self, url: &str, timeout: Duration) -> Result<PageState, BrowserError> {
        tracing::info!(%url, "Navigating");

        let cmd = command::page_navigate(url);
        let result = self.connection.send_command(cmd, timeout).await?;
        let _nav = command::parse_navigation_result(&result);

        // Wait briefly for the page to start loading, then get frame tree
        tokio::time::sleep(Duration::from_millis(500)).await;

        let frame_tree = self
            .connection
            .send_command(command::page_get_frame_tree(), timeout)
            .await?;
        let tree = command::parse_frame_tree(&frame_tree);
        let info = command::parse_page_info(&tree);

        // Try to get title via JS
        let title = self
            .connection
            .send_command(
                command::runtime_evaluate("document.title"),
                Duration::from_secs(5),
            )
            .await
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| info.title.clone());

        let dom_nodes = self
            .connection
            .send_command(
                command::runtime_evaluate("document.querySelectorAll('*').length"),
                Duration::from_secs(5),
            )
            .await
            .ok()
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let visible_text = self
            .connection
            .send_command(
                command::runtime_evaluate("document.body?.innerText?.slice(0, 2000) || ''"),
                Duration::from_secs(5),
            )
            .await
            .ok()
            .and_then(|v| {
                v.as_str().map(|s| {
                    s.lines()
                        .filter(|l| !l.trim().is_empty())
                        .map(String::from)
                        .take(50)
                        .collect::<Vec<_>>()
                })
            })
            .unwrap_or_default();

        *self.current_url.lock().await = info.url.clone();

        Ok(PageState {
            url: info.url,
            title,
            loaded: info.loaded,
            dom_node_count: dom_nodes,
            visible_text,
            timestamp: chrono::Utc::now(),
        })
    }

    async fn get_distilled_dom(&self, mode: DistillMode) -> Result<DistilledPage, BrowserError> {
        let doc_result = self
            .connection
            .send_command(command::dom_get_document(-1), Duration::from_secs(10))
            .await?;

        let title = self
            .connection
            .send_command(
                command::runtime_evaluate("document.title"),
                Duration::from_secs(3),
            )
            .await
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default();

        let url = self.current_url.lock().await.clone();
        let distiller = ans_distill::Distiller;
        Ok(distiller.process(&doc_result, mode, &url, &title))
    }

    async fn capture_screenshot(
        &self,
        format: ImageFormat,
        full_page: bool,
    ) -> Result<Vec<u8>, BrowserError> {
        let (fmt_str, quality) = match format {
            ImageFormat::Png => ("png", None),
            ImageFormat::Jpeg { quality } => ("jpeg", Some(quality)),
        };

        let mut params = serde_json::json!({ "format": fmt_str });
        if let Some(q) = quality {
            params["quality"] = serde_json::json!(q);
        }
        if full_page {
            params["captureBeyondViewport"] = serde_json::json!(true);
        }

        let cmd = CdpCommand::new("Page.captureScreenshot", params);
        let result = self
            .connection
            .send_command(cmd, Duration::from_secs(15))
            .await?;

        command::parse_screenshot_result(&result)
            .map_err(|e| BrowserError::CdpProtocol(e.to_string()))
    }

    async fn click(&self, selector: &str) -> Result<ActionOutcome, BrowserError> {
        let start = std::time::Instant::now();

        // 1. Get document root
        let doc_result = self
            .connection
            .send_command(command::dom_get_document(0), Duration::from_secs(5))
            .await?;
        let root_id = command::parse_dom_node_id(&doc_result)
            .map_err(|e| BrowserError::CdpProtocol(e.to_string()))?;

        // 2. Query selector
        let qs_result = self
            .connection
            .send_command(
                command::dom_query_selector(root_id, selector),
                Duration::from_secs(5),
            )
            .await
            .map_err(|e| {
                if matches!(e, BrowserError::CdpProtocol(_)) {
                    BrowserError::SelectorNotFound(selector.to_string())
                } else {
                    e
                }
            })?;

        let node_id = command::parse_query_selector_result(&qs_result)
            .map_err(|_| BrowserError::SelectorNotFound(selector.to_string()))?;

        // 3. Get box model for coordinates
        let box_result = self
            .connection
            .send_command(command::dom_get_box_model(node_id), Duration::from_secs(5))
            .await?;
        let box_model = command::parse_box_model(&box_result)
            .map_err(|_| BrowserError::NotInteractable(selector.to_string()))?;

        let mut center_x = box_model.x + box_model.width / 2.0;
        let mut center_y = box_model.y + box_model.height / 2.0;

        // Humanized: slight random offset from exact center + pre-click delay
        let humanize = self.stealth.as_ref().is_some_and(|s| s.humanize);
        if humanize {
            use ans_stealth::humanize;
            center_x += ans_stealth::humanize::position_jitter();
            center_y += ans_stealth::humanize::position_jitter();
            tokio::time::sleep(humanize::click_linger()).await;
        }

        // 4. Scroll element into view
        let _ = self
            .connection
            .send_command(
                command::dom_scroll_into_view(node_id),
                Duration::from_secs(5),
            )
            .await;

        // Brief delay for scroll animation
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 5. Dispatch mouse events
        if humanize {
            // Seek to element with movement, then press/release
            self.send_ignore(command::input_dispatch_mouse_event(
                "mouseMoved",
                center_x,
                center_y,
                "none",
            ))
            .await;
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        self.send_ignore(command::input_dispatch_mouse_event(
            "mousePressed",
            center_x,
            center_y,
            "left",
        ))
        .await;
        tokio::time::sleep(Duration::from_millis(
            if humanize { 60 } else { 50 },
        ))
        .await;
        self.send_ignore(command::input_dispatch_mouse_event(
            "mouseReleased",
            center_x,
            center_y,
            "left",
        ))
        .await;

        // ── Pre-click DOM snapshot ──────────────────────────────
        // Capture element count before the click so we can detect
        // whether the click actually changed the page (modals opening,
        // form fields appearing, tab switches — things that don't
        // necessarily change the URL).
        let dom_count_before = self
            .connection
            .send_command(
                command::runtime_evaluate("document.querySelectorAll('*').length"),
                Duration::from_secs(3),
            )
            .await
            .ok()
            .and_then(|r| command::parse_evaluate_result(&r).ok())
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let url_before = self.current_url.lock().await.clone();

        // Stability wait: let the page react to the click.
        // 500ms is enough for most interactions; the early-exit in
        // wait_for_stability returns sooner when readyState is already complete.
        Self::wait_for_stability(&self.connection, Duration::from_millis(500)).await;

        // ── Post-click DOM snapshot ─────────────────────────────
        let dom_count_after = self
            .connection
            .send_command(
                command::runtime_evaluate("document.querySelectorAll('*').length"),
                Duration::from_secs(3),
            )
            .await
            .ok()
            .and_then(|r| command::parse_evaluate_result(&r).ok())
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let url_after = self.current_url.lock().await.clone();
        let url_changed = url_before != url_after;
        let dom_changed = dom_count_before != dom_count_after;
        let page_changed = url_changed || dom_changed;

        let elapsed = start.elapsed();

        Ok(ActionOutcome {
            success: page_changed,
            error_message: if page_changed {
                None
            } else {
                Some(format!(
                    "click had no visible effect — URL unchanged, DOM element count unchanged ({})",
                    dom_count_before,
                ))
            },
            execution_time_ms: elapsed.as_millis() as u64,
            page_url_after: Some(url_after),
            dom_changed: page_changed,
        })
    }

    async fn type_text(&self, selector: &str, text: &str) -> Result<ActionOutcome, BrowserError> {
        let start = std::time::Instant::now();

        // Focus the element via JS instead of a full click().
        // click() does 5+ CDP round-trips + mouse events + 1.5s stability
        // wait — overkill for focusing an input. A single Runtime.evaluate
        // call with .focus() is ~50ms vs ~2-3s.
        let focus_script = format!(
            r#"document.querySelector('{}')?.focus()"#,
            escape_js_string(selector),
        );
        let _ = self
            .connection
            .send_command(
                command::runtime_evaluate(&focus_script),
                Duration::from_secs(3),
            )
            .await;

        // Humanized: type character-by-character with variable delays
        let humanize = self.stealth.as_ref().is_some_and(|s| s.humanize);
        if humanize {
            use ans_stealth::humanize;
            for (i, c) in text.chars().enumerate() {
                let ch = c.to_string();
                self.send_ignore(command::input_insert_text(&ch)).await;
                tokio::time::sleep(humanize::typing_delay(c, i)).await;
            }
        } else {
            self.send_ignore(command::input_insert_text(text)).await;
        }

        // Stability wait after typing — form fields may trigger validation,
        // autocomplete, or dynamic UI changes.
        Self::wait_for_stability(&self.connection, Duration::from_millis(300)).await;

        let elapsed = start.elapsed();

        Ok(ActionOutcome {
            success: true,
            error_message: None,
            execution_time_ms: elapsed.as_millis() as u64,
            page_url_after: Some(self.current_url.lock().await.clone()),
            dom_changed: true,
        })
    }

    async fn select_option(
        &self,
        selector: &str,
        value: &str,
    ) -> Result<ActionOutcome, BrowserError> {
        let start = std::time::Instant::now();

        // Use JS to set the select value and dispatch a change event
        let script = format!(
            r"
            (function() {{
                var el = document.querySelector('{}');
                if (!el) return 'selector not found';
                el.value = '{}';
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                return 'ok';
            }})()
            ",
            escape_js_string(selector),
            escape_js_string(value),
        );

        let cmd = command::runtime_evaluate(&script);
        let result = self
            .connection
            .send_command(cmd, Duration::from_secs(5))
            .await?;
        let eval_result = command::parse_evaluate_result(&result)
            .map_err(|e| BrowserError::CdpProtocol(e.to_string()))?;

        let success = eval_result.as_str() == Some("ok");
        let elapsed = start.elapsed();

        Ok(ActionOutcome {
            success,
            error_message: if success {
                None
            } else {
                eval_result.as_str().map(String::from)
            },
            execution_time_ms: elapsed.as_millis() as u64,
            page_url_after: Some(self.current_url.lock().await.clone()),
            dom_changed: true,
        })
    }

    async fn scroll(
        &self,
        pixels: Option<i32>,
        selector: Option<&str>,
    ) -> Result<ActionOutcome, BrowserError> {
        let start = std::time::Instant::now();

        let script = if let Some(sel) = selector {
            format!(
                "document.querySelector('{}')?.scrollIntoView({{ behavior: 'instant', block: 'center' }});",
                escape_js_string(sel)
            )
        } else if let Some(px) = pixels {
            format!("window.scrollBy(0, {px});")
        } else {
            "window.scrollBy(0, 300);".to_string()
        };

        let cmd = command::runtime_evaluate(&script);
        let _ = self
            .connection
            .send_command(cmd, Duration::from_secs(5))
            .await;

        let elapsed = start.elapsed();

        Ok(ActionOutcome {
            success: true,
            error_message: None,
            execution_time_ms: elapsed.as_millis() as u64,
            page_url_after: Some(self.current_url.lock().await.clone()),
            dom_changed: false,
        })
    }

    async fn wait_for_load(&self, timeout: Duration) -> Result<PageState, BrowserError> {
        // Poll document.readyState until complete or timeout
        let start = std::time::Instant::now();
        loop {
            let cmd = command::runtime_evaluate("document.readyState");
            if let Ok(result) = self
                .connection
                .send_command(cmd, Duration::from_secs(3))
                .await
            {
                if let Ok(val) = command::parse_evaluate_result(&result) {
                    if val.as_str() == Some("complete") {
                        break;
                    }
                }
            }

            if start.elapsed() > timeout {
                return Err(BrowserError::NavigationTimeout(timeout));
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        let current_url = self.current_url.lock().await.clone();
        Ok(PageState {
            url: current_url,
            title: String::new(),
            loaded: true,
            dom_node_count: 0,
            visible_text: vec![],
            timestamp: chrono::Utc::now(),
        })
    }

    async fn execute_script(&self, script: &str) -> Result<String, BrowserError> {
        let cmd = command::runtime_evaluate(script);
        let result = self
            .connection
            .send_command(cmd, Duration::from_secs(10))
            .await?;
        let val = command::parse_evaluate_result(&result)
            .map_err(|e| BrowserError::CdpProtocol(e.to_string()))?;
        Ok(val.to_string())
    }

    async fn close(&self) -> Result<(), BrowserError> {
        tracing::info!("Closing CDP backend");
        // The Chromium process is killed on drop, but we also close
        // the WebSocket gracefully by dropping the sender.
        // Process cleanup happens in ChromiumProcess::drop.
        Ok(())
    }
}

/// Escape a string for safe embedding in a JavaScript string literal.
fn escape_js_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
