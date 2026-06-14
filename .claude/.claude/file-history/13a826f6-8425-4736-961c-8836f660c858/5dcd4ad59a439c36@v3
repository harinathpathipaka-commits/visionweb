//! Immune system for the Agent Nervous System.
//!
//! Two components run at intake, before any eye perceives content:
//!
//! 1. **Distraction Classifier** (<1ms): Heuristic rules identify
//!    ads, popups, cookie banners, etc. and decide: dismiss, block,
//!    suppress, or ignore.
//!
//! 2. **Prompt Injection Detector** (<5ms): Regex + heuristics scan
//!    page content for instruction-override attacks via hidden divs,
//!    aria-labels, CSS pseudo-elements, zero-width chars, homoglyphs.
//!
//! Combined pipeline latency budget: <10ms p99.

pub mod classifier;
pub mod detector;
pub mod rules;
pub mod serialization;

pub use classifier::DistractionClassifier;
pub use detector::InjectionDetector;
