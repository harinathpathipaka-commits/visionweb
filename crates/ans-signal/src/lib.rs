//! Signal router.
//!
//! Receives [`EyeReport`]s from all eyes, scores each for goal relevance,
//! suppresses noise (low-relevance reports), amplifies goal signals,
//! resolves cross-eye contradictions, and produces a unified
//! [`RoutedSignal`] for the Decision Intelligence Layer.

pub mod contradiction;
pub mod router;

pub use contradiction::ContradictionResolver;
pub use router::SignalRouter;
