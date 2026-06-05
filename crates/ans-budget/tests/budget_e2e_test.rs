//! End-to-end budget circuit breaker tests.
//!
//! Verifies the full budget lifecycle across all 4 circuit breaker modes:
//! Normal → Conservative → Critical → Emergency
//!
//! Tests:
//! - Mode transitions at correct thresholds
//! - One-way constraint (never loosens within a session)
//! - Per-mode behavior: vision disabled, LLM disabled, verification throttling
//! - Budget status snapshot accuracy
//! - Reset on daily limit increase

use ans_budget::{BudgetTracker, CircuitBreaker};
use ans_budget::serialization::{budget_config_to_proto, budget_status_to_proto, proto_to_budget_config};
use ans_budget::tracker::BudgetConfig;
use ans_core::budget::BudgetMode;

// ── Circuit breaker mode transitions ─────────────────────────

#[test]
fn test_full_budget_lifecycle_all_modes() {
    let config = BudgetConfig {
        daily_spend_limit_cents: 1000,
        ..Default::default()
    };
    let mut tracker = BudgetTracker::new(config);

    // Start at Normal
    assert_eq!(tracker.mode(), BudgetMode::Normal);
    assert!(tracker.remaining_pct() > 90.0);

    // Spend 810 → 19% remaining → Conservative (10-20%)
    tracker.track_spend(810);
    assert_eq!(tracker.mode(), BudgetMode::Conservative);
    let pct = tracker.remaining_pct();
    assert!(pct > 10.0 && pct <= 20.0, "Expected 10-20%, got {pct}%");

    // Spend 100 more (total 910) → 9% remaining → Critical (5-10%)
    tracker.track_spend(100);
    assert_eq!(tracker.mode(), BudgetMode::Critical);
    let pct = tracker.remaining_pct();
    assert!(pct > 5.0 && pct <= 10.0, "Expected 5-10%, got {pct}%");

    // Spend 50 more (total 960) → 4% remaining → Emergency (<5%)
    tracker.track_spend(50);
    assert_eq!(tracker.mode(), BudgetMode::Emergency);
    let pct = tracker.remaining_pct();
    assert!(pct <= 5.0, "Expected <5%, got {pct}%");
}

#[test]
fn test_circuit_breaker_one_way_constraint() {
    let mut breaker = CircuitBreaker::new();
    assert_eq!(breaker.current_mode(), BudgetMode::Normal);

    // Tighten to Conservative
    breaker.update(15.0);
    assert_eq!(breaker.current_mode(), BudgetMode::Conservative);

    // Cannot go back to Normal even if remaining_pct "recovers"
    breaker.update(50.0);
    assert_eq!(breaker.current_mode(), BudgetMode::Conservative);

    // Tighten to Critical
    breaker.update(7.0);
    assert_eq!(breaker.current_mode(), BudgetMode::Critical);

    // Cannot go back
    breaker.update(100.0);
    assert_eq!(breaker.current_mode(), BudgetMode::Critical);

    // Tighten to Emergency
    breaker.update(3.0);
    assert_eq!(breaker.current_mode(), BudgetMode::Emergency);

    // Cannot go back
    breaker.update(100.0);
    assert_eq!(breaker.current_mode(), BudgetMode::Emergency);
}

#[test]
fn test_mode_from_remaining_pct_boundaries() {
    // Exact boundary values
    assert_eq!(BudgetMode::from_remaining_pct(20.1), BudgetMode::Normal);
    assert_eq!(BudgetMode::from_remaining_pct(20.0), BudgetMode::Conservative);
    assert_eq!(BudgetMode::from_remaining_pct(10.1), BudgetMode::Conservative);
    assert_eq!(BudgetMode::from_remaining_pct(10.0), BudgetMode::Critical);
    assert_eq!(BudgetMode::from_remaining_pct(5.1), BudgetMode::Critical);
    assert_eq!(BudgetMode::from_remaining_pct(5.0), BudgetMode::Emergency);
    assert_eq!(BudgetMode::from_remaining_pct(0.0), BudgetMode::Emergency);
}

// ── Mode behavior tests ──────────────────────────────────────

#[test]
fn test_normal_allows_all_calls() {
    let mode = BudgetMode::Normal;
    assert!(mode.allows_llm_call());
    assert!(mode.allows_vision());
    assert!(mode.should_verify_goal(0));
    assert!(mode.should_verify_goal(1));
    assert!(mode.should_verify_goal(42));
}

#[test]
fn test_conservative_allows_llm_not_vision() {
    let mode = BudgetMode::Conservative;
    assert!(mode.allows_llm_call());
    assert!(!mode.allows_vision());
    // Goal verification every 2nd action
    assert!(mode.should_verify_goal(0));
    assert!(!mode.should_verify_goal(1));
    assert!(mode.should_verify_goal(2));
}

#[test]
fn test_critical_disallows_llm_and_vision() {
    let mode = BudgetMode::Critical;
    assert!(!mode.allows_llm_call());
    assert!(!mode.allows_vision());
    // Goal verification every 3rd action
    assert!(mode.should_verify_goal(0));
    assert!(!mode.should_verify_goal(1));
    assert!(!mode.should_verify_goal(2));
    assert!(mode.should_verify_goal(3));
}

#[test]
fn test_emergency_blocks_all_llm() {
    let mode = BudgetMode::Emergency;
    assert!(!mode.allows_llm_call());
    assert!(!mode.allows_vision());
    assert!(!mode.should_verify_goal(0));
    assert!(!mode.should_verify_goal(1));
    assert!(!mode.should_verify_goal(100));
}

// ── Budget status snapshot ───────────────────────────────────

#[test]
fn test_budget_status_accuracy() {
    let config = BudgetConfig {
        daily_spend_limit_cents: 2000,
        ..Default::default()
    };
    let mut tracker = BudgetTracker::new(config);

    tracker.track_spend(500);
    let status = tracker.status();

    assert_eq!(status.mode, BudgetMode::Normal);
    assert_eq!(status.budget_cents, 2000);
    assert_eq!(status.spent_cents, 500);
    assert_eq!(status.daily_spend_limit_cents, 2000);
    assert_eq!(status.daily_spent_cents, 500);
    // 1500 / 2000 * 100 = 75%
    assert!(
        (status.remaining_pct - 75.0).abs() < 1.0,
        "Expected ~75%, got {}%",
        status.remaining_pct
    );
}

// ── Reset on limit increase ──────────────────────────────────

#[test]
fn test_configure_resets_on_limit_increase() {
    let mut tracker = BudgetTracker::new(BudgetConfig {
        daily_spend_limit_cents: 100,
        ..Default::default()
    });

    // Spend almost all budget
    tracker.track_spend(96);
    assert_eq!(tracker.mode(), BudgetMode::Emergency);

    // Increase limit → breaker resets
    tracker.configure(BudgetConfig {
        daily_spend_limit_cents: 500,
        ..Default::default()
    });
    assert_eq!(tracker.mode(), BudgetMode::Normal);
    // Spent 96 of 500 → ~80.8% remaining
    assert!(tracker.remaining_pct() > 80.0);
}

#[test]
fn test_configure_does_not_reset_on_limit_decrease() {
    let mut tracker = BudgetTracker::new(BudgetConfig {
        daily_spend_limit_cents: 500,
        ..Default::default()
    });

    tracker.track_spend(100); // 80% remaining → Normal
    assert_eq!(tracker.mode(), BudgetMode::Normal);

    // Decrease limit — no reset (limit didn't increase)
    tracker.configure(BudgetConfig {
        daily_spend_limit_cents: 200,
        ..Default::default()
    });
    // Still in Normal since limit wasn't increased
    assert_eq!(tracker.mode(), BudgetMode::Normal);
}

// ── Proto serialization round-trip ───────────────────────────

#[test]
fn test_budget_config_proto_roundtrip() {
    let config = BudgetConfig {
        default_per_goal_cents: 500,
        daily_spend_limit_cents: 5000,
        normal_threshold_pct: 20.0,
        conservative_threshold_pct: 10.0,
        critical_threshold_pct: 5.0,
    };

    let proto = budget_config_to_proto(&config);
    let roundtripped = proto_to_budget_config(&ans_proto::ans::BudgetConfigRequest {
        default_per_goal_cents: proto.default_per_goal_cents,
        daily_api_key_spend_limit_cents: proto.daily_api_key_spend_limit_cents,
        normal_threshold_pct: proto.normal_threshold_pct,
        conservative_threshold_pct: proto.conservative_threshold_pct,
        critical_threshold_pct: proto.critical_threshold_pct,
    });

    assert_eq!(roundtripped.default_per_goal_cents, config.default_per_goal_cents);
    assert_eq!(roundtripped.daily_spend_limit_cents, config.daily_spend_limit_cents);
}

#[test]
fn test_budget_status_proto_conversion() {
    let mut tracker = BudgetTracker::new(BudgetConfig::default());
    tracker.track_spend(4000); // Push into Conservative

    let status = tracker.status();
    let proto = budget_status_to_proto(&status, "goal-test-1");

    assert_eq!(proto.mode, "conservative");
    assert_eq!(proto.budget_cents, 5000);
    assert_eq!(proto.spent_cents, 4000);
    assert_eq!(proto.daily_spend_limit_cents, 5000);
    assert_eq!(proto.daily_spent_cents, 4000);
}

// ── Mode ordering (for circuit breaker comparison) ───────────

#[test]
fn test_budget_mode_ordering() {
    assert!(BudgetMode::Normal < BudgetMode::Conservative);
    assert!(BudgetMode::Conservative < BudgetMode::Critical);
    assert!(BudgetMode::Critical < BudgetMode::Emergency);
    assert!(BudgetMode::Normal < BudgetMode::Emergency);
}

// ── Edge cases ───────────────────────────────────────────────

#[test]
fn test_zero_spend_limit_triggers_emergency_on_spend() {
    let config = BudgetConfig {
        daily_spend_limit_cents: 0,
        ..Default::default()
    };
    let mut tracker = BudgetTracker::new(config);
    // Constructor doesn't update breaker, so mode starts Normal.
    // First spend triggers breaker update, which detects 0% remaining.
    tracker.track_spend(0);
    assert_eq!(tracker.remaining_pct(), 0.0);
    assert_eq!(tracker.mode(), BudgetMode::Emergency);
}

#[test]
fn test_spend_at_limit() {
    let config = BudgetConfig {
        daily_spend_limit_cents: 100,
        ..Default::default()
    };
    let mut tracker = BudgetTracker::new(config);
    tracker.track_spend(100);
    assert_eq!(tracker.remaining_pct(), 0.0);
    assert_eq!(tracker.mode(), BudgetMode::Emergency);
}

#[test]
fn test_spend_overflow_protection() {
    let config = BudgetConfig {
        daily_spend_limit_cents: 100,
        ..Default::default()
    };
    let mut tracker = BudgetTracker::new(config);
    // Spend more than limit — saturating_add prevents overflow
    tracker.track_spend(200);
    assert_eq!(tracker.remaining_pct(), 0.0);
    assert_eq!(tracker.mode(), BudgetMode::Emergency);
}
