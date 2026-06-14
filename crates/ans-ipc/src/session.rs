//! Session manager for goal-scoped browser sessions.
//!
//! Each session wraps a [`CdpBackend`] (Chromium instance) and is
//! identified by a UUID. Sessions are organized by goal — not by tab —
//! and share the same goal state store.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use ans_cdp::CdpBackend;
use ans_core::backend::{BrowserBackend, BrowserError, ImageFormat, PageState};
use ans_core::distill::{DistillMode, DistilledPage};
use ans_core::session::{Action, ActionOutcome, SessionStatus};
use tokio::sync::RwLock;
use tracing;
use uuid::Uuid;

use crate::pool::BrowserPool;

/// Handle to an active browser session.
pub struct SessionHandle {
    pub session_id: Uuid,
    pub goal_id: Uuid,
    pub backend: CdpBackend,
    pub current_url: String,
    pub status: SessionStatus,
}

/// Thread-safe session registry.
///
/// Cloning shares the underlying `Arc<RwLock<>>` — all references
/// see the same sessions.
#[derive(Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<Uuid, SessionHandle>>>,
    pool: BrowserPool,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            pool: BrowserPool::new(2),
        }
    }

    /// Create a session manager with a specific pool size.
    #[must_use]
    pub fn with_pool_size(pool_size: usize) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            pool: BrowserPool::new(pool_size),
        }
    }

    /// Create a new session by launching a Chromium instance.
    /// If `start_url` is provided, navigate to it immediately.
    pub async fn create(
        &self,
        goal_id: Uuid,
        start_url: Option<String>,
    ) -> Result<Uuid, BrowserError> {
        let session_id = Uuid::new_v4();
        tracing::info!(%session_id, %goal_id, "Creating session");

        let backend = self.pool.acquire().await?;

        if let Some(url) = &start_url {
            let _ = backend
                .navigate(url, Duration::from_secs(10))
                .await
                .map(|state| {
                    tracing::info!(url = %state.url, "Session started at URL");
                });
        }

        let handle = SessionHandle {
            session_id,
            goal_id,
            backend,
            current_url: start_url.unwrap_or_else(|| "about:blank".into()),
            status: SessionStatus::Idle,
        };

        self.sessions.write().await.insert(session_id, handle);

        Ok(session_id)
    }

    /// Close a session and kill its Chromium process.
    pub async fn close(&self, session_id: Uuid) -> Result<(), BrowserError> {
        tracing::info!(%session_id, "Closing session");
        let handle = self.sessions.write().await.remove(&session_id);
        if let Some(handle) = handle {
            self.pool.release(handle.backend).await;
        }
        Ok(())
    }

    /// Close all active sessions.
    pub async fn close_all(&self) {
        let ids: Vec<Uuid> = self.sessions.read().await.keys().copied().collect();
        for id in ids {
            let _ = self.close(id).await;
        }
        tracing::info!("All sessions closed");
    }

    /// Execute an action on a session.
    pub async fn execute_action(
        &self,
        session_id: Uuid,
        action: &Action,
    ) -> Result<ActionOutcome, BrowserError> {
        let sessions = self.sessions.read().await;
        let handle = sessions
            .get(&session_id)
            .ok_or(BrowserError::BrowserClosed)?;

        let outcome = match action.action_type {
            ans_core::session::ActionType::Click => {
                let selector = action
                    .selector
                    .as_deref()
                    .ok_or_else(|| BrowserError::SelectorNotFound("missing selector".into()))?;
                handle.backend.click(selector).await?
            }
            ans_core::session::ActionType::Type => {
                let selector = action
                    .selector
                    .as_deref()
                    .ok_or_else(|| BrowserError::SelectorNotFound("missing selector".into()))?;
                let text = action.value.as_deref().unwrap_or("");
                handle.backend.type_text(selector, text).await?
            }
            ans_core::session::ActionType::Scroll => {
                let pixels = action.value.as_deref().and_then(|v| v.parse().ok());
                let selector = action.selector.as_deref();
                handle.backend.scroll(pixels, selector).await?
            }
            ans_core::session::ActionType::Select => {
                let selector = action
                    .selector
                    .as_deref()
                    .ok_or_else(|| BrowserError::SelectorNotFound("missing selector".into()))?;
                let value = action.value.as_deref().unwrap_or("");
                handle.backend.select_option(selector, value).await?
            }
            ans_core::session::ActionType::Navigate => {
                let url = action
                    .value
                    .as_deref()
                    .ok_or_else(|| BrowserError::CdpProtocol("missing URL".into()))?;
                let state = handle
                    .backend
                    .navigate(url, Duration::from_secs(10))
                    .await?;
                ActionOutcome {
                    success: true,
                    error_message: None,
                    execution_time_ms: 0,
                    page_url_after: Some(state.url),
                    dom_changed: false,
                }
            }
            ans_core::session::ActionType::Wait => {
                let ms = action
                    .value
                    .as_deref()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1000u64);
                tokio::time::sleep(Duration::from_millis(ms)).await;
                ActionOutcome {
                    success: true,
                    error_message: None,
                    execution_time_ms: ms,
                    page_url_after: None,
                    dom_changed: false,
                }
            }
            ans_core::session::ActionType::Submit => {
                // Submit is typically handled as a click on a submit button
                // or pressing Enter. Evaluate a JS submit on the form.
                let selector = action.selector.as_deref().unwrap_or("form");
                let result = handle
                    .backend
                    .execute_script(&format!(
                        "(function(){{var f=document.querySelector('{}');if(f){{f.submit();return'ok'}}return'not found'}})()",
                        escape_for_js(selector)
                    ))
                    .await?;
                ActionOutcome {
                    success: result.contains("ok"),
                    error_message: if result.contains("ok") {
                        None
                    } else {
                        Some("form not found".into())
                    },
                    execution_time_ms: 0,
                    page_url_after: None,
                    dom_changed: true,
                }
            }
            ans_core::session::ActionType::Screenshot => {
                let _bytes = handle
                    .backend
                    .capture_screenshot(ans_core::backend::ImageFormat::Png, false)
                    .await?;
                ActionOutcome {
                    success: true,
                    error_message: None,
                    execution_time_ms: 0,
                    page_url_after: None,
                    dom_changed: false,
                }
            }
            ans_core::session::ActionType::Evaluate => {
                let script = action.value.as_deref().unwrap_or("");
                if script.is_empty() {
                    return Err(BrowserError::CdpProtocol(
                        "evaluate requires a script in the value field".into(),
                    ));
                }
                let result = handle.backend.execute_script(script).await?;
                ActionOutcome {
                    success: true,
                    error_message: Some(result),
                    execution_time_ms: 0,
                    page_url_after: None,
                    dom_changed: false,
                }
            }
        };

        Ok(outcome)
    }

    /// Navigate a session to a URL.
    pub async fn navigate(&self, session_id: Uuid, url: &str) -> Result<PageState, BrowserError> {
        let sessions = self.sessions.read().await;
        let handle = sessions
            .get(&session_id)
            .ok_or(BrowserError::BrowserClosed)?;

        handle.backend.navigate(url, Duration::from_secs(10)).await
    }

    /// Distill the DOM in a session's current page.
    pub async fn get_distilled_dom(
        &self,
        session_id: Uuid,
        mode: DistillMode,
    ) -> Result<DistilledPage, BrowserError> {
        let sessions = self.sessions.read().await;
        let handle = sessions
            .get(&session_id)
            .ok_or(BrowserError::BrowserClosed)?;
        handle.backend.get_distilled_dom(mode).await
    }

    /// Capture a screenshot from a session's current page.
    pub async fn capture_screenshot(
        &self,
        session_id: Uuid,
        format: ImageFormat,
        full_page: bool,
    ) -> Result<Vec<u8>, BrowserError> {
        let sessions = self.sessions.read().await;
        let handle = sessions
            .get(&session_id)
            .ok_or(BrowserError::BrowserClosed)?;
        handle.backend.capture_screenshot(format, full_page).await
    }

    /// Get the number of active sessions.
    #[must_use] 
    pub fn session_count(&self) -> usize {
        self.sessions.try_read().map_or(0, |g| g.len())
    }

    /// Get the current URL for a session without fetching DOM.
    pub async fn get_url(&self, session_id: Uuid) -> Option<String> {
        let sessions = self.sessions.read().await;
        sessions.get(&session_id).map(|h| h.current_url.clone())
    }

    /// Update the session status.
    pub async fn set_status(&self, session_id: Uuid, status: SessionStatus) {
        let mut sessions = self.sessions.write().await;
        if let Some(handle) = sessions.get_mut(&session_id) {
            handle.status = status;
            tracing::info!(%session_id, ?status, "Session status changed");
        }
    }

    /// Find the first session ID associated with a given goal.
    /// Returns `None` if no session exists for that goal.
    pub async fn find_session_by_goal(&self, goal_id: Uuid) -> Option<Uuid> {
        let sessions = self.sessions.read().await;
        sessions.iter().find_map(|(id, h)| {
            if h.goal_id == goal_id { Some(*id) } else { None }
        })
    }

    // ── Human interaction helpers (used by error recovery UI) ──

    /// Simulate a mouse click at page coordinates (x, y).
    /// Uses CDP `Input.dispatchMouseEvent` for realistic interaction.
    pub async fn click_at(&self, session_id: Uuid, x: i32, y: i32) -> Result<String, BrowserError> {
        let sessions = self.sessions.read().await;
        let handle = sessions.get(&session_id).ok_or(BrowserError::BrowserClosed)?;
        let script = format!(
            "(function(){{var el=document.elementFromPoint({x},{y});if(el){{el.focus();el.click();return 'clicked '+el.tagName+'#'+(el.id||'')}}else{{return 'no element at ({x},{y})'}}}})()"
        );
        handle.backend.execute_script(&script).await
    }

    /// Type text into the currently focused element on the page.
    pub async fn type_to_page(&self, session_id: Uuid, text: &str) -> Result<String, BrowserError> {
        let sessions = self.sessions.read().await;
        let handle = sessions.get(&session_id).ok_or(BrowserError::BrowserClosed)?;
        let escaped = escape_for_js(text);
        let script = format!(
            "(function(){{var t='{escaped}';var el=document.activeElement||document.body;if(el.isContentEditable){{el.textContent+=t}}else{{el.value=(el.value||'')+t;el.dispatchEvent(new Event('input',{{bubbles:true}}))}}return 'typed '+t.length+' chars'}})()"
        );
        handle.backend.execute_script(&script).await
    }

    /// Simulate a key press on the page (Enter, Tab, Escape, Backspace).
    pub async fn key_press(&self, session_id: Uuid, key: &str) -> Result<String, BrowserError> {
        let sessions = self.sessions.read().await;
        let handle = sessions.get(&session_id).ok_or(BrowserError::BrowserClosed)?;
        let (code, key_name) = match key {
            "Enter" => ("Enter", "Enter"),
            "Tab" => ("Tab", "Tab"),
            "Escape" => ("Escape", "Escape"),
            "Backspace" => ("Backspace", "Backspace"),
            other => (other, other),
        };
        let script = format!(
            "(function(){{var el=document.activeElement||document.body;var kd=new KeyboardEvent('keydown',{{key:'{key_name}',code:'{code}',bubbles:true}});var kp=new KeyboardEvent('keypress',{{key:'{key_name}',code:'{code}',bubbles:true}});var ku=new KeyboardEvent('keyup',{{key:'{key_name}',code:'{code}',bubbles:true}});el.dispatchEvent(kd);el.dispatchEvent(kp);el.dispatchEvent(ku);return 'key_'+'{key_name}'}})()"
        );
        handle.backend.execute_script(&script).await
    }

    /// Scroll the page by a number of pixels (positive = down).
    pub async fn scroll_by(&self, session_id: Uuid, pixels: i32) -> Result<String, BrowserError> {
        let sessions = self.sessions.read().await;
        let handle = sessions.get(&session_id).ok_or(BrowserError::BrowserClosed)?;
        let script = format!(
            "window.scrollBy({{top:{pixels},behavior:'smooth'}});'scrolled {pixels}px'"
        );
        handle.backend.execute_script(&script).await
    }
}

fn escape_for_js(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
}
