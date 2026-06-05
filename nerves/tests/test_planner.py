"""Tests for AgentPlanner — dual-mode action planning (cold + warm start)."""

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from ans_nerves.planner.planner import (
    AgentPlanner,
    PlannedAction,
    _best_recommendation,
    _classify_plan_source,
    _format_recommendations,
)
from ans_nerves.scoring.intelligence import ScoredAction


def _make_scored_action(action_type="click", selector="#btn", composite=0.85, **kw):
    return ScoredAction(
        id="abc123",
        action_type=action_type,
        selector=selector,
        tool=action_type,
        context_type="search_form",
        composite_score=composite,
        outcome_score=0.9,
        result_score=0.7,
        error_penalty=0.0,
        business_outcome=0.5,
        use_count=5,
        last_used_at=1700000000000,
        distance=0.15,
        **kw,
    )


class TestFormatRecommendations:
    def test_empty_list(self):
        assert _format_recommendations([]) == []

    def test_formats_scores_rounded(self):
        sa = _make_scored_action()
        recs = _format_recommendations([sa])
        assert len(recs) == 1
        assert recs[0]["action_type"] == "click"
        assert recs[0]["selector"] == "#btn"
        assert recs[0]["composite_score"] == 0.85
        assert "use_count" in recs[0]

    def test_max_five(self):
        actions = [_make_scored_action(selector=f"#{i}") for i in range(10)]
        recs = _format_recommendations(actions)
        assert len(recs) == 5


class TestBestRecommendation:
    def test_empty_returns_none(self):
        assert _best_recommendation([]) is None

    def test_low_score_returns_none(self):
        sa = _make_scored_action(composite=0.1)
        assert _best_recommendation([sa]) is None

    def test_returns_top_action(self):
        sa = _make_scored_action()
        result = _best_recommendation([sa])
        assert result is not None
        assert result.action_type == "click"
        assert result.selector == "#btn"
        assert result.source == "memory_validated"


class TestClassifyPlanSource:
    def test_memory_validated_when_match(self):
        actions = [_make_scored_action(action_type="click", selector="#btn")]
        parsed = {"action_type": "click", "selector": "#btn"}
        assert _classify_plan_source(parsed, actions) == "memory_validated"

    def test_memory_override_when_differs(self):
        actions = [_make_scored_action(action_type="click", selector="#btn")]
        parsed = {"action_type": "fill", "selector": "#input"}
        assert _classify_plan_source(parsed, actions) == "memory_override"

    def test_llm_cold_when_no_actions(self):
        parsed = {"action_type": "click", "selector": "#btn"}
        assert _classify_plan_source(parsed, []) == "llm_cold"


class TestAgentPlannerColdStart:
    def setup_method(self):
        self.planner = AgentPlanner()
        # Force cold start: zero records in LanceDB
        self.planner._intelligence = MagicMock(total_records=0)

    @pytest.mark.asyncio
    async def test_cold_start_with_no_records(self):
        """When LanceDB has 0 records, planner uses cold start (LLM reasoning)."""
        mock_response = MagicMock()
        mock_response.parsed = {
            "action_type": "click",
            "selector": "#search-btn",
            "value": "",
            "tool": "click",
            "reasoning": "The search button is visible and enabled. Clicking it submits the form.",
            "confidence": 0.9,
            "expected_outcome": "Search results page loads",
        }

        mock_client = MagicMock()
        mock_client.complete_structured = AsyncMock(return_value=mock_response)

        with patch("ans_nerves.planner.planner.get_llm_client", return_value=mock_client):
            action = await self.planner.plan_next_action(
                goal_context="Search for flights",
                sub_goal="Click the search button",
                unified_perception="Search form visible with From/To fields and Search button.",
                available_elements=[
                    {"selector": "#search-btn", "tag": "button", "text": "Search",
                     "aria_role": "button", "is_visible": True, "is_enabled": True},
                ],
            )

        assert action.action_type == "click"
        assert action.selector == "#search-btn"
        assert action.source == "llm_cold"
        assert action.confidence == 0.9

    @pytest.mark.asyncio
    async def test_cold_start_llm_exception_fallback(self):
        """When LLM fails during cold start, return a safe wait action."""
        mock_client = MagicMock()
        mock_client.complete_structured = AsyncMock(side_effect=RuntimeError("API down"))

        with patch("ans_nerves.planner.planner.get_llm_client", return_value=mock_client):
            action = await self.planner.plan_next_action(
                goal_context="Search for flights",
            )

        assert action.action_type == "wait"
        assert action.source == "fallback"
        assert action.confidence == 0.1

    @pytest.mark.asyncio
    async def test_cold_start_unparseable_fallback(self):
        """When LLM returns None parsed, return wait fallback."""
        mock_response = MagicMock()
        mock_response.parsed = None
        mock_response.content = "garbled"

        mock_client = MagicMock()
        mock_client.complete_structured = AsyncMock(return_value=mock_response)

        with patch("ans_nerves.planner.planner.get_llm_client", return_value=mock_client):
            action = await self.planner.plan_next_action(
                goal_context="Search for flights",
            )

        assert action.action_type == "wait"
        assert action.source == "fallback"


class TestAgentPlannerWarmStart:
    def setup_method(self):
        # Create a planner whose intelligence has >= 3 records for warm start
        mock_intel = MagicMock()
        mock_intel.total_records = 5
        mock_intel.query_best_actions = AsyncMock(return_value=[])
        self.planner = AgentPlanner(intelligence=mock_intel)
        self.mock_intel = mock_intel

    @pytest.mark.asyncio
    async def test_warm_start_validates_recommendation(self):
        """With 5 records, planner queries memory and validates with LLM."""
        scored = _make_scored_action(action_type="click", selector="#search-btn")
        self.mock_intel.query_best_actions.return_value = [scored]
        self.mock_intel.total_records = 5

        mock_response = MagicMock()
        mock_response.parsed = {
            "action_type": "click",
            "selector": "#search-btn",
            "value": "",
            "tool": "click",
            "reasoning": "Memory recommends clicking #search-btn (score 0.85). Validated: button is visible.",
            "confidence": 0.92,
            "expected_outcome": "Search results load",
        }

        mock_client = MagicMock()
        mock_client.complete_structured = AsyncMock(return_value=mock_response)

        with patch("ans_nerves.planner.planner.get_llm_client", return_value=mock_client):
            action = await self.planner.plan_next_action(
                goal_context="Search for flights",
                unified_perception="Search form ready.",
                page_type="search_form",
            )

        assert action.action_type == "click"
        assert action.source == "memory_validated"
        self.mock_intel.query_best_actions.assert_called_once()

    @pytest.mark.asyncio
    async def test_warm_start_llm_overrides_recommendation(self):
        """LLM can override memory when current page state contradicts."""
        scored = _make_scored_action(action_type="click", selector="#old-btn")
        self.mock_intel.query_best_actions.return_value = [scored]
        self.mock_intel.total_records = 5

        mock_response = MagicMock()
        mock_response.parsed = {
            "action_type": "dismiss_overlay",
            "selector": "#cookie-banner-close",
            "value": "",
            "tool": "dismiss_overlay",
            "reasoning": "Memory recommended #old-btn but a cookie banner is blocking everything. Dismissing first.",
            "confidence": 0.88,
            "expected_outcome": "Cookie banner closes, buttons become clickable.",
        }

        mock_client = MagicMock()
        mock_client.complete_structured = AsyncMock(return_value=mock_response)

        with patch("ans_nerves.planner.planner.get_llm_client", return_value=mock_client):
            action = await self.planner.plan_next_action(
                goal_context="Search for flights",
                page_type="search_form",
            )

        assert action.action_type == "dismiss_overlay"
        assert action.source == "memory_override"

    @pytest.mark.asyncio
    async def test_warm_start_llm_exception_uses_top_rec(self):
        """When LLM fails during warm start, fall back to top memory recommendation."""
        scored = _make_scored_action(action_type="click", selector="#search-btn")
        self.mock_intel.query_best_actions.return_value = [scored]
        self.mock_intel.total_records = 5

        mock_client = MagicMock()
        mock_client.complete_structured = AsyncMock(side_effect=RuntimeError("API down"))

        with patch("ans_nerves.planner.planner.get_llm_client", return_value=mock_client):
            action = await self.planner.plan_next_action(
                goal_context="Search for flights",
                page_type="search_form",
            )

        assert action.action_type == "click"
        assert action.selector == "#search-btn"
        assert action.source == "memory_validated"
