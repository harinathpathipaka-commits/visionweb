"""Advanced Multi-Factor Decision Scorer.

Scores every (action, tool, context) tuple across 6 dimensions:
1. Immediate outcome — Did the action work technically?
2. Goal advancement — Did it advance the goal?
3. Efficiency — How fast vs. expected? (sigmoid-scaled)
4. Consistency — How often does this action succeed in similar contexts?
5. Business impact — Long-term business value
6. Error penalty — Error severity via NLP matching (subtracted)

Weights are configurable. Defaults calibrated for browser-automation tasks.
"""

from __future__ import annotations

import math
import re
from dataclasses import dataclass, field
from typing import Any


# ── Error severity classification ────────────────────────────

_ERROR_SEVERITY_PATTERNS: list[tuple[float, str]] = [
    (1.0, r"captcha|recaptcha|bot.detect|are you a robot"),
    (0.95, r"paywall|premium required|subscribe to continue|payment required"),
    (0.90, r"login required|sign in to continue|authentication required"),
    (0.85, r"access denied|forbidden|403|unauthorized|401"),
    (0.80, r"not found|404|page does not exist|dead link"),
    (0.75, r"timeout|timed out|took too long|connection reset"),
    (0.60, r"element not found|no such element|selector.*not found|unable to locate"),
    (0.55, r"not clickable|element not interactable|not visible|hidden"),
    (0.50, r"stale element|detached from dom|page has changed"),
    (0.40, r"blocked|overlay|modal|popup.*block"),
    (0.30, r"server error|500|502|503|internal server error"),
    (0.25, r"navigation.*fail|redirect loop|too many redirects"),
    (0.20, r"unexpected.*state|wrong page|page mismatch"),
    (0.15, r"silent.*fail|no effect|nothing changed|no visible change"),
]


def classify_error_severity(error_message: str) -> float:
    """Classify error severity 0-1 via regex patterns.

    Returns 0.0 for empty/None messages (no error occurred).
    Returns 0.0–1.0 based on pattern matching, higher = more severe.
    """
    if not error_message:
        return 0.0

    msg_lower = error_message.lower().strip()
    best_score = 0.0

    for score, pattern in _ERROR_SEVERITY_PATTERNS:
        if re.search(pattern, msg_lower):
            if score > best_score:
                best_score = score

    return best_score if best_score > 0.0 else 0.35  # generic unknown error = 0.35


# ── Sigmoid efficiency ────────────────────────────────────────


def sigmoid_efficiency(
    execution_time_ms: int,
    expected_ms: int = 2000,
    steepness: float = 0.002,
) -> float:
    """Score execution efficiency via sigmoid: fast → 1.0, slow → ~0.0.

    A 200ms click gets ~0.85. A 5000ms timeout gets ~0.05.
    Centre point (0.5) is at expected_ms.
    """
    if execution_time_ms <= 0 or expected_ms <= 0:
        return 1.0
    # Logistic function centred on expected_ms
    x = steepness * (expected_ms - execution_time_ms)
    return 1.0 / (1.0 + math.exp(-x))


# ── Outcome classes ───────────────────────────────────────────


@dataclass
class FactorScores:
    """Per-dimension scores, each in [0, 1]."""

    immediate: float = 0.0
    goal: float = 0.0
    efficiency: float = 0.0
    consistency: float = 0.5  # neutral default
    business: float = 0.0
    error_penalty: float = 0.0  # subtracted, so store as positive [0,1]

    def to_dict(self) -> dict[str, float]:
        return {
            "immediate": self.immediate,
            "goal": self.goal,
            "efficiency": self.efficiency,
            "consistency": self.consistency,
            "business": self.business,
            "error_penalty": self.error_penalty,
        }


@dataclass
class ScoringWeights:
    """Configurable dimension weights. Must sum to ~1.0."""

    immediate: float = 0.30
    goal: float = 0.25
    efficiency: float = 0.10
    consistency: float = 0.15
    business: float = 0.10
    error: float = 0.10  # penalty weight (subtracted)

    def validate(self) -> None:
        total = sum([
            self.immediate, self.goal, self.efficiency,
            self.consistency, self.business, self.error,
        ])
        if abs(total - 1.0) > 0.06:
            raise ValueError(f"Weights sum to {total:.3f}, expected ~1.0")

    def to_dict(self) -> dict[str, float]:
        return {
            "immediate": self.immediate,
            "goal": self.goal,
            "efficiency": self.efficiency,
            "consistency": self.consistency,
            "business": self.business,
            "error": self.error,
        }


# ── Main scorer ───────────────────────────────────────────────


class AdvancedMultiFactorScorer:
    """Scores actions across 6 dimensions with configurable weights.

    Usage:
        scorer = AdvancedMultiFactorScorer()

        score, details = scorer.score(
            action_succeeded=True,
            goal_advanced=True,
            sub_goal_completed=False,
            execution_time_ms=450,
            expected_time_ms=2000,
            error_message=None,
            past_success_rate=0.85,
            business_outcome=0.0,
        )
    """

    def __init__(
        self,
        weights: ScoringWeights | None = None,
    ) -> None:
        self.weights = weights or ScoringWeights()
        self.weights.validate()

    # ── Public API ─────────────────────────────────────────────

    def score(
        self,
        *,
        action_succeeded: bool = False,
        results_produced: str = "",
        goal_advanced: bool = False,
        sub_goal_completed: bool = False,
        criterion_met: bool = False,
        execution_time_ms: int = 0,
        expected_time_ms: int = 2000,
        error_message: str | None = None,
        past_success_rate: float = 0.5,
        business_outcome: float = 0.0,
    ) -> tuple[float, FactorScores]:
        """Compute composite score across all 6 dimensions.

        Returns (composite_score, factor_scores).
        composite_score is in [0, 1] range (clamped).
        """
        factors = FactorScores(
            immediate=self._score_immediate(action_succeeded, results_produced),
            goal=self._score_goal(goal_advanced, sub_goal_completed, criterion_met),
            efficiency=sigmoid_efficiency(execution_time_ms, expected_time_ms),
            consistency=past_success_rate,
            business=business_outcome,
            error_penalty=classify_error_severity(error_message or ""),
        )

        w = self.weights
        composite = (
            w.immediate * factors.immediate
            + w.goal * factors.goal
            + w.efficiency * factors.efficiency
            + w.consistency * factors.consistency
            + w.business * factors.business
            - w.error * factors.error_penalty
        )

        return max(0.0, min(1.0, composite)), factors

    # ── Dimension scorers ─────────────────────────────────────

    @staticmethod
    def _score_immediate(succeeded: bool, results: str) -> float:
        """Immediate technical outcome.

        success → 1.0
        partial results → 0.3–0.5
        failure → 0.0
        """
        if succeeded:
            return 1.0
        if results and len(results.strip()) > 10:
            return 0.4  # produced some output but failed
        return 0.0

    @staticmethod
    def _score_goal(
        goal_advanced: bool,
        sub_goal_completed: bool,
        criterion_met: bool,
    ) -> float:
        """Goal advancement granular scoring."""
        if sub_goal_completed:
            return 1.0
        if goal_advanced:
            return 0.7
        if criterion_met:
            return 0.4
        return 0.0

    # ── Online adaptation ─────────────────────────────────────

    def adapt_weights(
        self,
        factors: FactorScores,
        learning_rate: float = 0.01,
    ) -> ScoringWeights:
        """Online weight adaptation via gradient-ascent on factor scores.

        Dimensions that consistently produce high scores get higher weight.
        Dimensions that produce low scores (penalties) get lower weight.
        Call this periodically as more outcomes are logged.
        """
        factor_dict = factors.to_dict()
        # Map factor names to scoring weight names
        mapping = {
            "immediate": "immediate",
            "goal": "goal",
            "efficiency": "efficiency",
            "consistency": "consistency",
            "business": "business",
            "error_penalty": "error",
        }

        new_weights = self.weights.to_dict()
        for fname, wname in mapping.items():
            score = factor_dict[fname]
            # Boost weights for high-scoring dimensions, reduce for low
            delta = learning_rate * (score - 0.5) * 2.0
            new_weights[wname] = max(0.01, min(0.50, new_weights[wname] + delta))

        # Renormalise to sum to 1.0
        total = sum(new_weights.values())
        for k in new_weights:
            new_weights[k] /= total

        self.weights = ScoringWeights(**new_weights)
        return self.weights


# ── Legacy alias ──────────────────────────────────────────────
# Keep backward compat for DecisionIntelligence which imports DecisionScorer
DecisionScorer = AdvancedMultiFactorScorer
