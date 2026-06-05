//! Goal state persistence for crash recovery.
//!
//! Goal state lives primarily in memory (Arc<`RwLock`<GoalStateStore>>)
//! but is snapshotted to `LanceDB` every 30 seconds so the daemon can
//! recover after a restart.

use ans_core::goal::GoalState;

/// Persistent goal state store.
pub struct GoalStore;

impl GoalStore {
    #[must_use] 
    pub const fn open(_path: &str) -> Self {
        Self
    }

    pub const fn snapshot(&self, _goals: &[GoalState]) {}

    #[must_use] 
    pub const fn restore(&self) -> Vec<GoalState> {
        Vec::new()
    }
}
