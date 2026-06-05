//! 4-level cost circuit breaker.
//!
//! Transitions are one-way within a session (tightening only):
//! Normal → Conservative → Critical → Emergency
//!
//! The circuit breaker RESETS to Normal when a new session starts
//! (fresh budget). It does not auto-recover within a session.

use ans_core::budget::BudgetMode;

/// The circuit breaker state machine.
pub struct CircuitBreaker {
    mode: BudgetMode,
}

impl CircuitBreaker {
    #[must_use] 
    pub const fn new() -> Self {
        Self {
            mode: BudgetMode::Normal,
        }
    }

    #[must_use] 
    pub const fn current_mode(&self) -> BudgetMode {
        self.mode
    }

    pub fn update(&mut self, remaining_pct: f32) {
        let new_mode = BudgetMode::from_remaining_pct(remaining_pct);
        if new_mode > self.mode {
            self.mode = new_mode;
        }
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}
