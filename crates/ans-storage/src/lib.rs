//! `LanceDB` storage layer.
//!
//! Embedded columnar + vector database running inside the Rust process.
//! Zero network, zero config — reads/writes Arrow columns directly.
//!
//! Responsibilities:
//! - [`DecisionRecord`] CRUD with vector indexing (cosine similarity)
//! - Goal state persistence (crash recovery)
//! - Session history archival
//! - `IVF_PQ` index for sub-20ms search at 1M+ records

pub mod decisions;
pub mod goals;
pub mod index;
pub mod serialization;

pub use decisions::DecisionStore;
pub use goals::GoalStore;
