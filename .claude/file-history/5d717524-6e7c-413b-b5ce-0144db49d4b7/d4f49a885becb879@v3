"""Tests for AdvancedMultiFactorScorer and scoring helpers."""

import pytest

from ans_nerves.scoring.scorer import (
    AdvancedMultiFactorScorer,
    DecisionScorer,
    FactorScores,
    ScoringWeights,
    classify_error_severity,
    sigmoid_efficiency,
)


class TestClassifyErrorSeverity:
    def test_empty_string_returns_zero(self):
        assert classify_error_severity("") == 0.0

    def test_none_returns_zero(self):
        assert classify_error_severity(None) == 0.0  # type: ignore[arg-type]

    def test_captcha_is_max_severity(self):
        assert classify_error_severity("Captcha detected") == 1.0

    def test_recaptcha_is_max_severity(self):
        assert classify_error_severity("recaptcha challenge appeared") == 1.0

    def test_bot_detect_is_max_severity(self):
        assert classify_error_severity("are you a robot check failed") == 1.0

    def test_paywall(self):
        assert classify_error_severity("premium required to continue") == 0.95

    def test_access_denied(self):
        assert classify_error_severity("access denied: 403 Forbidden") == 0.85

    def test_timeout(self):
        assert classify_error_severity("request timed out after 30s") == 0.75

    def test_element_not_found(self):
        assert classify_error_severity("no such element: #search-btn") == 0.60

    def test_not_clickable(self):
        assert classify_error_severity("element not interactable at point") == 0.55

    def test_stale_element(self):
        assert classify_error_severity("stale element reference: detached from dom") == 0.50

    def test_unknown_error_defaults_to_035(self):
        assert classify_error_severity("something weird happened") == 0.35

    def test_case_insensitive(self):
        # "ELEMENT NOT FOUND" matches both "element not found" (0.60)
        # and "not found" (0.80) — highest severity wins
        assert classify_error_severity("ELEMENT NOT FOUND") == 0.80

    def test_matches_highest_severity(self):
        """When multiple patterns match, return the highest severity."""
        # "blocked by captcha" matches both blocked(0.40) and captcha(1.0)
        assert classify_error_severity("blocked by captcha") == 1.0


class TestSigmoidEfficiency:
    def test_instant_is_near_one(self):
        score = sigmoid_efficiency(1, expected_ms=2000)
        assert score > 0.95

    def test_at_expected_is_05(self):
        score = sigmoid_efficiency(2000, expected_ms=2000)
        assert abs(score - 0.5) < 0.01

    def test_slow_is_near_zero(self):
        score = sigmoid_efficiency(10000, expected_ms=2000)
        assert score < 0.05

    def test_zero_time_returns_one(self):
        score = sigmoid_efficiency(0, expected_ms=2000)
        assert score == 1.0

    def test_custom_steepness(self):
        flat = sigmoid_efficiency(3000, expected_ms=2000, steepness=0.0001)
        steep = sigmoid_efficiency(3000, expected_ms=2000, steepness=0.01)
        assert flat > steep  # flat curve penalises less


class TestScoringWeights:
    def test_defaults_sum_to_one(self):
        w = ScoringWeights()
        total = sum(w.to_dict().values())
        assert abs(total - 1.0) < 0.01

    def test_invalid_sum_raises(self):
        w = ScoringWeights(immediate=0.9, goal=0.9)
        with pytest.raises(ValueError):
            w.validate()

    def test_to_dict(self):
        w = ScoringWeights()
        d = w.to_dict()
        assert "immediate" in d
        assert "error" in d
        assert len(d) == 6


class TestFactorScores:
    def test_defaults(self):
        f = FactorScores()
        assert f.immediate == 0.0
        assert f.consistency == 0.5  # neutral default

    def test_to_dict(self):
        f = FactorScores(immediate=1.0, goal=0.7)
        d = f.to_dict()
        assert d["immediate"] == 1.0
        assert d["goal"] == 0.7
        assert len(d) == 6


class TestAdvancedMultiFactorScorer:
    def setup_method(self):
        self.scorer = AdvancedMultiFactorScorer()

    def test_perfect_action(self):
        composite, factors = self.scorer.score(
            action_succeeded=True,
            results_produced="clicked search button",
            goal_advanced=True,
            sub_goal_completed=True,
            execution_time_ms=200,
            expected_time_ms=2000,
            error_message=None,
            past_success_rate=0.9,
            business_outcome=0.8,
        )
        assert composite > 0.80
        assert factors.immediate == 1.0
        assert factors.goal == 1.0
        assert factors.error_penalty == 0.0

    def test_total_failure(self):
        composite, factors = self.scorer.score(
            action_succeeded=False,
            results_produced="",
            goal_advanced=False,
            error_message="element not found: #missing-btn",
        )
        assert composite < 0.30
        assert factors.immediate == 0.0
        assert factors.goal == 0.0
        assert factors.error_penalty > 0.0

    def test_partial_results(self):
        """Failed but produced some output."""
        _, factors = self.scorer.score(
            action_succeeded=False,
            results_produced="partial form data extracted and saved",
        )
        assert factors.immediate == 0.4

    def test_sub_goal_completed_returns_max_goal_score(self):
        _, factors = self.scorer.score(
            sub_goal_completed=True,
        )
        assert factors.goal == 1.0

    def test_goal_advanced_no_completion(self):
        _, factors = self.scorer.score(
            goal_advanced=True,
            sub_goal_completed=False,
        )
        assert factors.goal == 0.7

    def test_criterion_met_only(self):
        _, factors = self.scorer.score(
            criterion_met=True,
        )
        assert factors.goal == 0.4

    def test_error_severity_reduces_composite(self):
        composite_clean, _ = self.scorer.score(
            action_succeeded=True,
            goal_advanced=True,
        )
        composite_error, _ = self.scorer.score(
            action_succeeded=True,
            goal_advanced=True,
            error_message="captcha challenge appeared",
        )
        assert composite_error < composite_clean

    def test_custom_weights(self):
        w = ScoringWeights(
            immediate=0.50, goal=0.20, efficiency=0.05,
            consistency=0.10, business=0.10, error=0.05,
        )
        scorer = AdvancedMultiFactorScorer(weights=w)
        composite, _ = scorer.score(
            action_succeeded=True,
            goal_advanced=True,
            sub_goal_completed=True,
        )
        assert composite >= 0.80

    def test_adapt_weights_converges(self):
        """Repeated adaptation on high-scoring factors increases those weights."""
        initial_immediate = self.scorer.weights.immediate
        factors = FactorScores(
            immediate=1.0, goal=0.7, efficiency=0.8,
            consistency=0.9, business=0.5, error_penalty=0.0,
        )
        for _ in range(20):
            self.scorer.adapt_weights(factors, learning_rate=0.05)
        assert self.scorer.weights.immediate > initial_immediate

    def test_adapted_weights_stay_normalized(self):
        factors = FactorScores(immediate=1.0)
        self.scorer.adapt_weights(factors, learning_rate=0.1)
        total = sum(self.scorer.weights.to_dict().values())
        assert abs(total - 1.0) < 0.01

    def test_legacy_alias(self):
        assert DecisionScorer is AdvancedMultiFactorScorer

    def test_composite_clamped_to_zero(self):
        composite, _ = self.scorer.score(
            action_succeeded=False,
            error_message="captcha detected — bot check failed",
            past_success_rate=0.0,
        )
        assert composite >= 0.0

    def test_composite_clamped_to_one(self):
        composite, _ = self.scorer.score(
            action_succeeded=True,
            results_produced="done",
            goal_advanced=True,
            sub_goal_completed=True,
            execution_time_ms=1,
            past_success_rate=1.0,
            business_outcome=1.0,
        )
        assert composite <= 1.0
