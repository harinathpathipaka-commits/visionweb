use std::collections::HashMap;

/// What the agent is currently doing (or not doing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SessionStatus {
    Idle,
    Navigating,
    Executing,
    Perceiving,
    Blocked,
    Failed,
}

/// An action the agent wants to execute.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Action {
    pub action_type: ActionType,
    pub selector: Option<String>,
    pub value: Option<String>,
    pub params: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ActionType {
    Click,
    Type,
    Scroll,
    Select,
    Navigate,
    Wait,
    Submit,
    Screenshot,
    /// Execute arbitrary JavaScript in the page context.
    /// Value = script, selector = optional CSS selector for scoping (unused for now).
    /// Result is JSON-serialized in `ActionOutcome.error_message` on success.
    Evaluate,
}

impl ActionType {
    #[must_use] 
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Click => "click",
            Self::Type => "type",
            Self::Scroll => "scroll",
            Self::Select => "select",
            Self::Navigate => "navigate",
            Self::Wait => "wait",
            Self::Submit => "submit",
            Self::Screenshot => "screenshot",
            Self::Evaluate => "evaluate",
        }
    }
}

/// The immediate result of executing an action.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActionOutcome {
    pub success: bool,
    pub error_message: Option<String>,
    pub execution_time_ms: u64,
    pub page_url_after: Option<String>,
    pub dom_changed: bool,
}
