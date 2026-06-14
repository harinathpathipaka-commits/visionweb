/// Cost circuit breaker mode.
///
/// Every LLM/Vision API call checks the current budget before executing.
/// The mode determines which calls are allowed and which are skipped.
///
/// Transitions are one-way (tightening only) within a session:
/// ```text
/// Normal → Conservative → Critical → Emergency
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum BudgetMode {
    /// >20% budget remaining. All eyes active, no throttling.
    Normal,
    /// 10-20% remaining. Vision Eye on every 3rd action. Goal Verifier
    /// throttled. LLM calls use smaller models when available.
    Conservative,
    /// 5-10% remaining. Vision Eye OFF (DOM-only mode). Goal Verifier
    /// on every 3rd action. Only critical LLM calls allowed.
    Critical,
    /// <5% remaining. ALL LLM calls OFF. Pure DOM heuristic mode.
    /// Decision scoring from memory only (no LLM scoring).
    Emergency,
}

impl BudgetMode {
    /// Determine the mode for the given remaining budget percentage.
    #[must_use] 
    pub fn from_remaining_pct(remaining_pct: f32) -> Self {
        if remaining_pct > 20.0 {
            Self::Normal
        } else if remaining_pct > 10.0 {
            Self::Conservative
        } else if remaining_pct > 5.0 {
            Self::Critical
        } else {
            Self::Emergency
        }
    }

    /// Can we make an LLM call in this mode?
    #[must_use] 
    pub const fn allows_llm_call(&self) -> bool {
        matches!(self, Self::Normal | Self::Conservative)
    }

    /// Can we call the Vision Eye in this mode?
    #[must_use] 
    pub const fn allows_vision(&self) -> bool {
        matches!(self, Self::Normal)
    }

    /// Should we skip goal verification for this action?
    #[must_use] 
    pub const fn should_verify_goal(&self, action_index: usize) -> bool {
        match self {
            Self::Normal => true,
            Self::Conservative => action_index % 2 == 0,
            Self::Critical => action_index % 3 == 0,
            Self::Emergency => false,
        }
    }
}
