//! Goal state manager.
//!
//! Owns the shared [`GoalState`] for every active goal, protected by
//! `Arc<RwLock<GoalStateStore>>`. All sessions working on the same goal
//! read/write this shared state. Progress updates are broadcast via
//! `tokio::broadcast` to all subscribers.

pub mod manager;
pub mod serialization;
pub mod store;

pub use manager::GoalManager;
pub use store::{GoalStateNotification, GoalStateStore, GoalStoreError};
