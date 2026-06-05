//! Internal pub/sub via `tokio::broadcast`.
//!
//! Fan-out for goal updates and eye reports within the Rust process.
//! Python clients subscribe via gRPC streaming, which wraps these
//! broadcast channels.

use tokio::sync::broadcast;
use uuid::Uuid;

/// A goal progress update pushed to all subscribers.
#[derive(Debug, Clone)]
pub struct GoalUpdate {
    pub goal_id: Uuid,
    pub progress: f32,
    pub status: String,
    pub message: String,
}

/// A report from one of the 5 Eyes.
#[derive(Debug, Clone)]
pub struct EyeReportEvent {
    pub eye_name: String,
    pub goal_id: Uuid,
    pub confidence: f32,
    pub content: String,
}

/// Fan-out bus for goal state updates.
pub struct GoalEventBus {
    tx: broadcast::Sender<GoalUpdate>,
}

impl GoalEventBus {
    #[must_use] 
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Subscribe to goal updates. Returns a receiver that sees all
    /// future publishes.
    #[must_use] 
    pub fn subscribe(&self) -> broadcast::Receiver<GoalUpdate> {
        self.tx.subscribe()
    }

    /// Publish an update to all subscribers.
    pub fn publish(&self, update: GoalUpdate) {
        let _ = self.tx.send(update);
    }
}

/// Fan-out bus for eye reports.
pub struct EyeReportBus {
    tx: broadcast::Sender<EyeReportEvent>,
}

impl EyeReportBus {
    #[must_use] 
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    #[must_use] 
    pub fn subscribe(&self) -> broadcast::Receiver<EyeReportEvent> {
        self.tx.subscribe()
    }

    pub fn publish(&self, event: EyeReportEvent) {
        let _ = self.tx.send(event);
    }
}

/// Combined event bus for the daemon.
pub struct EventBus {
    pub goals: GoalEventBus,
    pub eye_reports: EyeReportBus,
}

impl EventBus {
    #[must_use] 
    pub fn new() -> Self {
        Self {
            goals: GoalEventBus::new(256),
            eye_reports: EyeReportBus::new(256),
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
