//! ans-bench: Performance benchmarks for the Agent Nervous System.
//!
//! Benchmarks are organized by concern:
//! - `throughput`: DOM distillation, diff, and immune scanning throughput
//! - `concurrent`: Concurrent session and operation scaling

/// Re-export so benches can import from this crate.
pub use ans_core;
pub use ans_diff;
pub use ans_distill;
pub use ans_immune;
