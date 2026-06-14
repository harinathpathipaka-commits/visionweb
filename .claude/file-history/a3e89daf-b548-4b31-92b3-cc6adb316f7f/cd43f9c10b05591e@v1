//! Thread-safe goal state store with broadcast notifications.
//!
//! Wraps `Arc<RwLock<HashMap<Uuid, GoalState>>>` with `tokio::broadcast`
//! on every state mutation. All sessions working on the same goal share
//! this store. Cheap to clone (Arc pointer copy only).

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use ans_core::goal::{GoalState, GoalStatus, SubGoalStatus};
use tokio::sync::broadcast;
use tracing;
use uuid::Uuid;

/// Notification sent to subscribers when a goal's state changes.
#[derive(Debug, Clone)]
pub struct GoalStateNotification {
    pub goal_id: Uuid,
    pub progress: f32,
    pub status: GoalStatus,
    pub message: String,
}

/// Shared goal state store with pub/sub.
///
/// Cloning shares the underlying state — use this to hand out
/// references to the gRPC server, workers, and subscribers.
pub struct GoalStateStore {
    inner: Arc<RwLock<HashMap<Uuid, GoalState>>>,
    tx: broadcast::Sender<GoalStateNotification>,
}

impl GoalStateStore {
    /// Create an empty goal store with room for `capacity` notification
    /// listeners before slow consumers are dropped.
    #[must_use] 
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            tx,
        }
    }

    /// Insert a new goal state. Returns error if `goal_id` already exists.
    pub fn insert(&self, state: GoalState) -> Result<(), GoalStoreError> {
        let goal_id = state.goal_id;
        let progress = state.progress;
        let status = state.status;

        let mut map = self
            .inner
            .write()
            .map_err(|_| GoalStoreError::LockPoisoned)?;
        if map.contains_key(&goal_id) {
            return Err(GoalStoreError::DuplicateGoal(goal_id));
        }
        map.insert(goal_id, state);

        let _ = self.tx.send(GoalStateNotification {
            goal_id,
            progress,
            status,
            message: "Goal created".into(),
        });

        tracing::info!(%goal_id, "Goal inserted");
        Ok(())
    }

    /// Retrieve a goal state by ID.
    #[must_use] 
    pub fn get(&self, goal_id: &Uuid) -> Option<GoalState> {
        self.inner
            .read()
            .ok()
            .and_then(|map| map.get(goal_id).cloned())
    }

    /// Update an existing goal state. Fails if goal doesn't exist.
    pub fn update(&self, goal_id: Uuid, state: GoalState) -> Result<(), GoalStoreError> {
        let progress = state.progress;
        let status = state.status;

        let mut map = self
            .inner
            .write()
            .map_err(|_| GoalStoreError::LockPoisoned)?;
        if !map.contains_key(&goal_id) {
            return Err(GoalStoreError::NotFound(goal_id));
        }
        map.insert(goal_id, state);

        let _ = self.tx.send(GoalStateNotification {
            goal_id,
            progress,
            status,
            message: "Goal updated".into(),
        });

        tracing::debug!(%goal_id, %progress, "Goal state updated");
        Ok(())
    }

    /// Update progress and status for an existing goal. Lightweight; doesn't
    /// require constructing a full `GoalState`.
    pub fn update_progress(
        &self,
        goal_id: Uuid,
        progress: f32,
        status: GoalStatus,
    ) -> Result<GoalState, GoalStoreError> {
        let mut map = self
            .inner
            .write()
            .map_err(|_| GoalStoreError::LockPoisoned)?;
        let state = map
            .get_mut(&goal_id)
            .ok_or(GoalStoreError::NotFound(goal_id))?;

        state.progress = progress.clamp(0.0, 1.0);
        state.status = status;
        state.updated_at = chrono::Utc::now();

        if progress >= 1.0 {
            state.status = GoalStatus::Completed;
        }

        let snapshot = state.clone();

        let _ = self.tx.send(GoalStateNotification {
            goal_id,
            progress: state.progress,
            status: state.status,
            message: format!("Progress: {:.0}%", state.progress * 100.0),
        });

        tracing::info!(%goal_id, progress = %state.progress, "Goal progress updated");
        Ok(snapshot)
    }

    /// Replace all sub-goals (e.g., from Python LLM decomposer).
    pub fn update_sub_goals(
        &self,
        goal_id: Uuid,
        sub_goals: Vec<ans_core::goal::SubGoal>,
    ) -> Result<GoalState, GoalStoreError> {
        let mut map = self.inner.write().map_err(|_| GoalStoreError::LockPoisoned)?;
        let state = map.get_mut(&goal_id).ok_or(GoalStoreError::NotFound(goal_id))?;
        let total = sub_goals.len() as f32;
        let done = sub_goals.iter().filter(|s| s.status == SubGoalStatus::Done).count() as f32;
        let progress = if total > 0.0 { done / total } else { 0.0 };
        state.sub_goals = sub_goals;
        state.progress = progress;
        state.updated_at = chrono::Utc::now();
        let snapshot = state.clone();
        let _ = self.tx.send(GoalStateNotification {
            goal_id, progress: state.progress, status: state.status,
            message: format!("Sub-goals updated: {} total", total),
        });
        tracing::info!(%goal_id, sub_goal_count = total, "Goal sub-goals updated from decomposer");
        Ok(snapshot)
    }

    /// Remove a goal from the store.
    pub fn remove(&self, goal_id: &Uuid) -> Option<GoalState> {
        let mut map = self.inner.write().ok()?;
        let removed = map.remove(goal_id);

        if removed.is_some() {
            let _ = self.tx.send(GoalStateNotification {
                goal_id: *goal_id,
                progress: 1.0,
                status: GoalStatus::Completed,
                message: "Goal removed".into(),
            });
            tracing::info!(%goal_id, "Goal removed");
        }
        removed
    }

    /// List all active goal IDs.
    #[must_use] 
    pub fn list_ids(&self) -> Vec<Uuid> {
        self.inner
            .read()
            .map(|map| map.keys().copied().collect())
            .unwrap_or_default()
    }

    /// Number of active goals.
    #[must_use] 
    pub fn len(&self) -> usize {
        self.inner.read().map_or(0, |m| m.len())
    }

    /// Returns true if the store is empty.
    #[must_use] 
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Subscribe to goal state change notifications.
    #[must_use] 
    pub fn subscribe(&self) -> broadcast::Receiver<GoalStateNotification> {
        self.tx.subscribe()
    }
}

impl Default for GoalStateStore {
    fn default() -> Self {
        Self::new(256)
    }
}

impl Clone for GoalStateStore {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            tx: self.tx.clone(),
        }
    }
}

/// Errors that can occur during store operations.
#[derive(Debug, thiserror::Error)]
pub enum GoalStoreError {
    #[error("goal {0} already exists")]
    DuplicateGoal(Uuid),

    #[error("goal {0} not found")]
    NotFound(Uuid),

    #[error("lock poisoned — store is in an unrecoverable state")]
    LockPoisoned,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ans_core::goal::GoalContext;

    fn make_goal(id: Uuid, desc: &str) -> GoalState {
        GoalState {
            goal_id: id,
            description: desc.into(),
            status: GoalStatus::Active,
            progress: 0.0,
            sub_goals: vec![],
            context: GoalContext {
                current_url: None,
                last_action: None,
                last_observation: None,
                intent_embedding: vec![],
                distraction_count: 0,
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_insert_and_get() {
        let store = GoalStateStore::new(8);
        let id = Uuid::new_v4();
        let goal = make_goal(id, "Find flights to NYC");

        store.insert(goal).unwrap();
        let retrieved = store.get(&id).unwrap();
        assert_eq!(retrieved.description, "Find flights to NYC");
    }

    #[test]
    fn test_update_progress_triggers_notification() {
        let store = GoalStateStore::new(8);
        let id = Uuid::new_v4();
        store.insert(make_goal(id, "Test goal")).unwrap();

        let mut rx = store.subscribe();
        store.update_progress(id, 0.5, GoalStatus::Active).unwrap();

        let notification = rx.try_recv().unwrap();
        assert_eq!(notification.goal_id, id);
        assert!((notification.progress - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_duplicate_insert_fails() {
        let store = GoalStateStore::new(8);
        let id = Uuid::new_v4();
        store.insert(make_goal(id, "First")).unwrap();
        assert!(store.insert(make_goal(id, "Second")).is_err());
    }

    #[test]
    fn test_remove() {
        let store = GoalStateStore::new(8);
        let id = Uuid::new_v4();
        store.insert(make_goal(id, "Test")).unwrap();
        assert!(store.get(&id).is_some());
        store.remove(&id);
        assert!(store.get(&id).is_none());
    }

    #[test]
    fn test_completed_on_full_progress() {
        let store = GoalStateStore::new(8);
        let id = Uuid::new_v4();
        store.insert(make_goal(id, "Test")).unwrap();

        let updated = store.update_progress(id, 1.0, GoalStatus::Active).unwrap();
        assert_eq!(updated.status, GoalStatus::Completed);
    }
}
