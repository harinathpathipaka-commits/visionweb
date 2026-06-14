"""Tests for Cross-Eye Coordinator."""

from unittest.mock import MagicMock, AsyncMock, patch

import pytest

from ans_nerves.coordinator.coordinator import CrossEyeCoordinator, RoutedSignal
from ans_nerves.eyes.base import EyeReport


class TestCrossEyeCoordinator:
    def setup_method(self):
        self.coordinator = CrossEyeCoordinator()

    @pytest.mark.asyncio
    async def test_empty_reports(self):
        result = await self.coordinator.synthesize([])
        assert result.confidence == 0.0
        assert "No eye reports available" in result.unified_perception

    @pytest.mark.asyncio
    async def test_fallback_with_dom_reader(self):
        """Fallback synthesis when no LLM available — uses deterministic merge."""
        reports = [
            EyeReport(
                eye_name="dom_reader",
                confidence=0.9,
                goal_relevance=0.8,
                content={"interactive": ["#btn"], "distraction_flags": []},
            ),
            EyeReport(
                eye_name="vision",
                confidence=0.85,
                goal_relevance=0.7,
                content={"page_type": "search", "overlays": [], "anomalies": []},
            ),
        ]
        result = self.coordinator._fallback_synthesize(reports)
        assert "DOM:" in result.unified_perception
        assert "Vision:" in result.unified_perception
        assert result.confidence == 0.9

    @pytest.mark.asyncio
    async def test_fallback_with_all_eyes(self):
        reports = [
            EyeReport(eye_name="dom_reader", confidence=0.9, goal_relevance=0.8,
                      content={"interactive": [], "distraction_flags": []}),
            EyeReport(eye_name="vision", confidence=0.7, goal_relevance=0.6,
                      content={"page_type": "search", "overlays": [], "anomalies": []}),
            EyeReport(eye_name="page_diff", confidence=0.8, goal_relevance=0.4,
                      content={"summary": "minor_change"}),
            EyeReport(eye_name="goal_verifier", confidence=0.5, goal_relevance=0.9,
                      content={"criteria_met": False}),
            EyeReport(eye_name="error_detector", confidence=0.6, goal_relevance=0.3,
                      content={"failure_type": "none", "description": ""}),
        ]
        result = self.coordinator._fallback_synthesize(reports)
        assert "DOM:" in result.unified_perception
        assert "Vision:" in result.unified_perception
        assert "Diff:" in result.unified_perception
        assert "Verification:" in result.unified_perception
        assert "Error:" not in result.unified_perception  # no failure

    @pytest.mark.asyncio
    async def test_fallback_with_error(self):
        reports = [
            EyeReport(eye_name="dom_reader", confidence=0.5, goal_relevance=0.1,
                      content={"interactive": [], "distraction_flags": []}),
            EyeReport(eye_name="error_detector", confidence=0.9, goal_relevance=1.0,
                      content={"failure_type": "timeout", "description": "Request timed out"}),
        ]
        result = self.coordinator._fallback_synthesize(reports)
        assert "Error:" in result.unified_perception

    @pytest.mark.asyncio
    async def test_fallback_silent_fail_is_omitted(self):
        """silent_fail without description should be omitted."""
        reports = [
            EyeReport(eye_name="dom_reader", confidence=0.5, goal_relevance=0.1,
                      content={"interactive": [], "distraction_flags": []}),
            EyeReport(eye_name="error_detector", confidence=0.9, goal_relevance=1.0,
                      content={"failure_type": "silent_fail", "description": ""}),
        ]
        result = self.coordinator._fallback_synthesize(reports)
        assert "Error:" not in result.unified_perception

    @pytest.mark.asyncio
    async def test_fallback_error_with_description(self):
        """error_detector with failure + description should appear in output."""
        reports = [
            EyeReport(eye_name="error_detector", confidence=0.9, goal_relevance=1.0,
                      content={"failure_type": "crash", "description": "Browser crashed"}),
        ]
        result = self.coordinator._fallback_synthesize(reports)
        assert "Error:" in result.unified_perception
        assert "Browser crashed" in result.unified_perception

    @pytest.mark.asyncio
    async def test_fallback_error_without_description(self):
        """error_detector with failure but no description should still appear."""
        reports = [
            EyeReport(eye_name="error_detector", confidence=0.9, goal_relevance=1.0,
                      content={"failure_type": "timeout", "description": ""}),
        ]
        result = self.coordinator._fallback_synthesize(reports)
        assert "Error: timeout" in result.unified_perception

    @pytest.mark.asyncio
    async def test_fallback_with_anomalies(self):
        """Vision eye with anomalies should append Anomalies line."""
        reports = [
            EyeReport(eye_name="vision", confidence=0.7, goal_relevance=0.6,
                      content={"page_type": "search", "overlays": ["popup"], "anomalies": ["form_missing"]}),
        ]
        result = self.coordinator._fallback_synthesize(reports)
        assert "Anomalies:" in result.unified_perception
        assert "form_missing" in result.unified_perception


class TestSynthesizeWithLLM:
    """Tests for synthesize() with mocked LLM — covers the LLM success/fallback paths."""

    def setup_method(self):
        self.coordinator = CrossEyeCoordinator()
        self.reports = [
            EyeReport(
                eye_name="dom_reader", confidence=0.9, goal_relevance=0.8,
                content={"interactive": ["#btn"], "distraction_flags": []},
            ),
            EyeReport(
                eye_name="vision", confidence=0.85, goal_relevance=0.7,
                content={"page_type": "search", "overlays": [], "anomalies": []},
            ),
        ]

    @pytest.mark.asyncio
    async def test_synthesize_llm_success(self):
        mock_response = MagicMock()
        mock_response.parsed = {
            "unified_perception": "Search form is visible and ready.",
            "confidence": 0.92,
            "recommended_action_hint": "Type departure city",
            "contradictions": [],
            "alerts": [],
        }

        mock_client = MagicMock()
        mock_client.complete_structured = AsyncMock(return_value=mock_response)

        with patch("ans_nerves.coordinator.coordinator.get_llm_client", return_value=mock_client):
            result = await self.coordinator.synthesize(self.reports, goal_context="book a flight")
            assert result.confidence == 0.92
            assert "Search form" in result.unified_perception
            assert result.recommended_action_hint == "Type departure city"

    @pytest.mark.asyncio
    async def test_synthesize_llm_exception_falls_back(self):
        mock_client = MagicMock()
        mock_client.complete_structured = AsyncMock(side_effect=Exception("API error"))

        with patch("ans_nerves.coordinator.coordinator.get_llm_client", return_value=mock_client):
            result = await self.coordinator.synthesize(self.reports)
            assert "DOM:" in result.unified_perception
            assert "Vision:" in result.unified_perception

    @pytest.mark.asyncio
    async def test_synthesize_llm_none_parsed_falls_back(self):
        mock_response = MagicMock()
        mock_response.parsed = None

        mock_client = MagicMock()
        mock_client.complete_structured = AsyncMock(return_value=mock_response)

        with patch("ans_nerves.coordinator.coordinator.get_llm_client", return_value=mock_client):
            result = await self.coordinator.synthesize(self.reports)
            assert "DOM:" in result.unified_perception
            assert "Vision:" in result.unified_perception

    @pytest.mark.asyncio
    async def test_synthesize_with_contradictions(self):
        mock_response = MagicMock()
        mock_response.parsed = {
            "unified_perception": "Conflicting views detected.",
            "confidence": 0.7,
            "recommended_action_hint": None,
            "contradictions": [{"topic": "page_state", "conflicting_eyes": ["dom", "vision"]}],
            "alerts": [{"severity": "warning", "message": "captcha detected"}],
        }

        mock_client = MagicMock()
        mock_client.complete_structured = AsyncMock(return_value=mock_response)

        with patch("ans_nerves.coordinator.coordinator.get_llm_client", return_value=mock_client):
            result = await self.coordinator.synthesize(self.reports)
            assert len(result.contradictions) == 1
            assert len(result.alerts) == 1


class TestRoutedSignal:
    def test_defaults(self):
        rs = RoutedSignal()
        assert rs.confidence == 1.0
        assert rs.recommended_action_hint is None
        assert rs.contradictions == []
        assert rs.alerts == []

    def test_full_signal(self):
        rs = RoutedSignal(
            unified_perception="Search form visible",
            confidence=0.9,
            recommended_action_hint="Type departure city",
            contradictions=[{"topic": "page_state", "conflicting_eyes": ["dom", "vision"]}],
            alerts=[{"severity": "warning", "message": "captcha detected"}],
        )
        assert rs.confidence == 0.9
        assert len(rs.contradictions) == 1
        assert len(rs.alerts) == 1
