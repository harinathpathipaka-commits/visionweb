//! Page diff engine.
//!
//! Computes structural diffs between two [`DistilledPage`] snapshots.
//! Uses Zhang-Shasha tree edit distance on the distilled DOM tree.
//! Runs automatically after every action, before the Goal Verifier
//! checks sub-goal criteria.

pub mod engine;
pub mod types;

pub use engine::PageDiffer;
