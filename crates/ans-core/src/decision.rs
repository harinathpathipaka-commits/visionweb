use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::session::Action;

/// The Decision Intelligence Layer's unit of memory.
///
/// After every action executes, the scoring engine produces one of these.
/// It's stored in `LanceDB` and queried by cosine similarity on
/// `context_embedding` when the agent faces a similar situation again.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DecisionRecord {
    /// Unique record identifier.
    pub id: Uuid,
    /// Session this action was part of.
    pub session_id: Uuid,
    /// Goal this action was advancing.
    pub goal_id: Uuid,
    /// The action that was executed.
    pub action: Action,
    /// The tool used (e.g. "`cdp_click`", "`vision_verify`").
    pub tool: String,
    /// Task-type tag for grouping similar contexts.
    pub context_type: String,
    /// 768-dim (or 384-dim) embedding of the context at decision time.
    pub context_embedding: Vec<f32>,

    /// 3-layer business outcome.
    pub business_outcome: BusinessOutcome,

    /// Did the action itself succeed (no error from CDP)?
    pub outcome_success: bool,
    /// What did the action produce? (structured summary)
    pub results_summary: String,
    /// Error message if the action failed.
    pub error_message: Option<String>,
    /// Composite score: f(outcome, results, errors, `business_outcome`).
    pub composite_score: f32,

    /// How many times this (action, tool, `context_type`) combination has been used.
    pub use_count: u32,
    /// When this record was created.
    pub created_at: DateTime<Utc>,
    /// When this record was last updated (e.g. short-term / long-term outcome arrived).
    pub updated_at: DateTime<Utc>,
}

/// 3-layer temporal business outcome model.
///
/// Each layer answers a different question:
/// - **Immediate** (t=0): Did the action work technically?
/// - **Short-term** (t=minutes-hours): Did the goal advance in this session?
/// - **Long-term** (t=days-months): Did the business outcome materialize?
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BusinessOutcome {
    /// Immediate technical outcome (0.0 = failure, 1.0 = success).
    pub immediate: ImmediateOutcome,
    /// Short-term session outcome (arrives minutes to hours later).
    pub short_term: Option<ShortTermOutcome>,
    /// Long-term business impact (arrives days to months later).
    pub long_term: Option<LongTermOutcome>,
}

impl BusinessOutcome {
    /// Create a new outcome with only the immediate layer populated.
    #[must_use] 
    pub fn new_immediate(success: bool, error: Option<String>) -> Self {
        Self {
            immediate: ImmediateOutcome {
                action_succeeded: success,
                results_produced: String::new(),
                error_message: error,
                execution_time_ms: 0,
            },
            short_term: None,
            long_term: None,
        }
    }

    /// Weighted composite across all populated layers.
    #[must_use] 
    pub fn composite_score(&self) -> f32 {
        let immediate_weight = 0.6;
        let short_term_weight = 0.3;
        let long_term_weight = 0.1;

        let immediate_score = self.immediate.score();
        let mut total = immediate_score * immediate_weight;
        let mut weight_sum = immediate_weight;

        if let Some(ref st) = self.short_term {
            total += st.score() * short_term_weight;
            weight_sum += short_term_weight;
        }
        if let Some(ref lt) = self.long_term {
            total += lt.score() * long_term_weight;
            weight_sum += long_term_weight;
        }

        total / weight_sum
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImmediateOutcome {
    pub action_succeeded: bool,
    pub results_produced: String,
    pub error_message: Option<String>,
    pub execution_time_ms: u64,
}

impl ImmediateOutcome {
    #[must_use] 
    pub fn score(&self) -> f32 {
        if self.action_succeeded {
            1.0
        } else {
            // Penalize based on error severity (heuristic).
            match &self.error_message {
                Some(e) if e.contains("timeout") => 0.2,
                Some(e) if e.contains("not found") => 0.1,
                Some(_) => 0.0,
                None => 0.5,
            }
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ShortTermOutcome {
    pub goal_advanced: bool,
    /// Which verifiable criterion was met (if any).
    pub criterion_met: Option<String>,
    pub sub_goal_completed: bool,
    /// External source (e.g. "webhook:ci", "`goal_verifier`").
    pub source: String,
    pub timestamp: DateTime<Utc>,
}

impl ShortTermOutcome {
    #[must_use] 
    pub const fn score(&self) -> f32 {
        if self.sub_goal_completed {
            return 1.0;
        }
        if self.goal_advanced {
            return 0.7;
        }
        if self.criterion_met.is_some() {
            return 0.4;
        }
        0.0
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LongTermOutcome {
    pub business_goal_achieved: bool,
    /// e.g. "`ci_build_passed`", "`ticket_resolved`", "`customer_retained`".
    pub metric_name: String,
    pub metric_value: f32,
    /// External source (e.g. "webhook:crm", "api:analytics").
    pub source: String,
    pub timestamp: DateTime<Utc>,
}

impl LongTermOutcome {
    #[must_use] 
    pub const fn score(&self) -> f32 {
        if self.business_goal_achieved {
            1.0
        } else {
            self.metric_value.clamp(0.0, 1.0)
        }
    }
}

/// A ring buffer of recent decisions for in-memory access.
pub type DecisionBuffer = VecDeque<DecisionRecord>;

/// Maximum decisions kept in the in-memory buffer per session.
pub const DECISION_BUFFER_CAPACITY: usize = 100;
