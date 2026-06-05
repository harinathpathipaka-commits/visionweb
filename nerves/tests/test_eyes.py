"""Tests for the 5 Eyes."""

import json
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from ans_nerves.eyes.base import EyeReport
from ans_nerves.eyes.dom_reader import DomReaderEye, _classify_element, _is_noise
from ans_nerves.eyes.error_detector import ErrorDetectorEye
from ans_nerves.eyes.goal_verifier import GoalVerifierEye
from ans_nerves.eyes.page_diff import PageDiffEye
from ans_nerves.eyes.vision import VisionEye


# ── DOM Reader Eye (pure Python, no LLM) ──────────────────


class TestDomReader:
    def setup_method(self):
        self.eye = DomReaderEye()

    @pytest.mark.asyncio
    async def test_no_distilled_dom(self):
        report = await self.eye.observe("s1", {})
        assert report.confidence == 0.0
        assert "error" in report.content

    @pytest.mark.asyncio
    async def test_with_distilled_dom(self):
        dom = {
            "elements": [
                {
                    "tag": "button", "selector": "#search-btn",
                    "text": "Search", "css_class": "", "css_id": "",
                    "aria_role": "button", "is_visible": True, "is_enabled": True,
                    "goal_relevance_score": 0.9, "bounding_box": None,
                },
                {
                    "tag": "div", "selector": "#ad-banner",
                    "text": "Buy now", "css_class": "advertisement",
                    "css_id": "", "aria_role": "",
                    "is_visible": True, "is_enabled": True,
                    "goal_relevance_score": 0.1, "bounding_box": None,
                },
                {
                    "tag": "h1", "selector": "#title",
                    "text": "Welcome", "css_class": "", "css_id": "",
                    "aria_role": "", "is_visible": True, "is_enabled": True,
                    "goal_relevance_score": 0.5, "bounding_box": None,
                },
            ],
            "mode": "all_fields",
            "url": "https://example.com",
            "title": "Example",
        }
        report = await self.eye.observe("s1", {"distilled_dom": dom})
        assert report.confidence == 1.0
        assert len(report.content["interactive"]) == 1
        assert report.content["interactive"][0]["element_type"] == "button"
        assert len(report.content["distraction_flags"]) == 1
        assert len(report.content["semantic_blocks"]) == 1

    @pytest.mark.asyncio
    async def test_distilled_dom_as_json_string(self):
        dom = {
            "elements": [
                {"tag": "a", "selector": "#link", "text": "Click here",
                 "css_class": "", "css_id": "", "aria_role": "link",
                 "is_visible": True, "is_enabled": True,
                 "goal_relevance_score": 0.7, "bounding_box": None},
            ],
            "mode": "text_only", "url": "", "title": "",
        }
        report = await self.eye.observe("s1", {"distilled_dom": json.dumps(dom)})
        assert report.confidence == 1.0
        assert len(report.content["interactive"]) == 1

    @pytest.mark.asyncio
    async def test_invalid_json_string(self):
        report = await self.eye.observe("s1", {"distilled_dom": "not valid json{"})
        assert report.confidence == 0.0
        assert "Invalid" in report.content.get("error", "")


class TestElementClassification:
    def test_is_noise_ad_class(self):
        assert _is_noise({"css_class": "top-advertisement", "css_id": "", "aria_role": ""})

    def test_is_noise_cookie_class(self):
        assert _is_noise({"css_class": "", "css_id": "cookie-banner", "aria_role": ""})

    def test_is_noise_clean_element(self):
        assert not _is_noise({"css_class": "main-content", "css_id": "", "aria_role": "main"})

    def test_classify_button(self):
        assert _classify_element({"tag": "button", "css_class": "", "css_id": "", "aria_role": ""}) == "interactive"

    def test_classify_input(self):
        assert _classify_element({"tag": "input", "css_class": "", "css_id": "", "aria_role": ""}) == "interactive"

    def test_classify_aria_button(self):
        assert _classify_element({"tag": "div", "css_class": "", "css_id": "", "aria_role": "button"}) == "interactive"

    def test_classify_semantic_h1(self):
        assert _classify_element({"tag": "h1", "css_class": "", "css_id": "", "aria_role": ""}) == "semantic"

    def test_classify_noise_overrides(self):
        assert _classify_element({"tag": "button", "css_class": "popup-ad", "css_id": "", "aria_role": ""}) == "noise"


# ── Vision Eye (LLM-dependent — test edge cases only) ────


class TestVisionEye:
    def setup_method(self):
        self.eye = VisionEye()

    @pytest.mark.asyncio
    async def test_no_screenshot(self):
        report = await self.eye.observe("s1", {})
        assert report.confidence == 0.0
        assert report.content["page_type"] == "unknown"
        assert "No screenshot" in report.content["error"]

    def test_compute_confidence_unknown(self):
        assert self.eye._compute_confidence({"page_type": "unknown"}) == 0.5

    def test_compute_confidence_with_elements(self):
        assert self.eye._compute_confidence({
            "page_type": "search", "visible_elements": ["input", "button"],
        }) == 0.9


# ── Page Diff Eye (LLM-dependent — test edge cases only) ──


class TestPageDiffEye:
    def setup_method(self):
        self.eye = PageDiffEye()

    @pytest.mark.asyncio
    async def test_no_diff_data(self):
        report = await self.eye.observe("s1", {})
        assert report.confidence == 1.0
        assert report.content["summary"] == "no_change"


# ── Goal Verifier Eye (LLM-dependent — test edge cases only) ──


class TestGoalVerifierEye:
    def setup_method(self):
        self.eye = GoalVerifierEye()

    @pytest.mark.asyncio
    async def test_missing_sub_goal(self):
        report = await self.eye.observe("s1", {})
        assert report.confidence == 0.0
        assert "Missing" in report.content["reasoning"]

    @pytest.mark.asyncio
    async def test_missing_criteria(self):
        report = await self.eye.observe("s1", {"sub_goal_description": "Find flights"})
        assert report.confidence == 0.0


# ── Error Detector Eye (LLM-dependent — test edge cases only) ──


class TestErrorDetectorEye:
    def setup_method(self):
        self.eye = ErrorDetectorEye()

    @pytest.mark.asyncio
    async def test_no_action_description(self):
        report = await self.eye.observe("s1", {})
        assert report.confidence == 0.0
        assert "No action_description" in report.content["description"]


# ── LLM integration tests (mocked) ──────────────────────────


class TestVisionEyeLLM:
    def setup_method(self):
        self.eye = VisionEye()

    @pytest.mark.asyncio
    async def test_llm_success(self):
        mock_response = MagicMock()
        mock_response.parsed = {
            "page_type": "search",
            "visible_elements": ["input", "button"],
            "overlays": [],
            "blocked_regions": [],
            "anomalies": [],
        }

        mock_client = MagicMock()
        mock_client.complete_vision = AsyncMock(return_value=mock_response)

        with patch("ans_nerves.eyes.vision.get_llm_client", return_value=mock_client):
            report = await self.eye.observe("s1", {
                "screenshot_base64": "fakebase64==",
                "distilled_dom": {"elements": []},
                "goal_context": "Find flights",
            })
            assert report.confidence == 0.9
            assert report.content["page_type"] == "search"
            assert len(report.content["visible_elements"]) == 2

    @pytest.mark.asyncio
    async def test_llm_exception_falls_back(self):
        mock_client = MagicMock()
        mock_client.complete_vision = AsyncMock(side_effect=RuntimeError("API down"))

        with patch("ans_nerves.eyes.vision.get_llm_client", return_value=mock_client):
            report = await self.eye.observe("s1", {
                "screenshot_base64": "fakebase64==",
                "distilled_dom": {"elements": []},
            })
            assert report.confidence == 0.0
            assert report.content["error"] == "LLM call failed"

    @pytest.mark.asyncio
    async def test_llm_none_parsed_falls_back(self):
        mock_response = MagicMock()
        mock_response.parsed = None
        mock_response.content = "garbled response"

        mock_client = MagicMock()
        mock_client.complete_vision = AsyncMock(return_value=mock_response)

        with patch("ans_nerves.eyes.vision.get_llm_client", return_value=mock_client):
            report = await self.eye.observe("s1", {
                "screenshot_base64": "fakebase64==",
                "distilled_dom": {"elements": []},
            })
            assert report.confidence == 0.3
            assert "raw_response" in report.content


class TestPageDiffEyeLLM:
    def setup_method(self):
        self.eye = PageDiffEye()

    @pytest.mark.asyncio
    async def test_llm_success(self):
        mock_response = MagicMock()
        mock_response.parsed = {
            "summary": "search_results_updated",
            "what_changed": "New flights appeared",
            "is_goal_relevant": True,
            "new_elements_of_interest": ["#flight-123"],
            "confidence": 0.9,
        }

        mock_client = MagicMock()
        mock_client.complete_structured = AsyncMock(return_value=mock_response)

        with patch("ans_nerves.eyes.page_diff.get_llm_client", return_value=mock_client):
            report = await self.eye.observe("s1", {
                "diff": {"added": 1, "removed": 0},
                "goal_context": "Find flights",
            })
            assert report.confidence == 0.9
            assert report.content["summary"] == "search_results_updated"
            assert report.content["is_goal_relevant"] is True

    @pytest.mark.asyncio
    async def test_llm_exception_falls_back(self):
        mock_client = MagicMock()
        mock_client.complete_structured = AsyncMock(side_effect=RuntimeError("API down"))

        with patch("ans_nerves.eyes.page_diff.get_llm_client", return_value=mock_client):
            report = await self.eye.observe("s1", {"diff": {"added": 1}})
            assert report.confidence == 0.3
            assert report.content["error"] == "LLM call failed"

    @pytest.mark.asyncio
    async def test_llm_none_parsed_falls_back(self):
        mock_response = MagicMock()
        mock_response.parsed = None
        mock_response.content = "garbled"

        mock_client = MagicMock()
        mock_client.complete_structured = AsyncMock(return_value=mock_response)

        with patch("ans_nerves.eyes.page_diff.get_llm_client", return_value=mock_client):
            report = await self.eye.observe("s1", {"diff": {"added": 1}})
            assert report.confidence == 0.3
            assert "raw_response" in report.content


class TestGoalVerifierEyeLLM:
    def setup_method(self):
        self.eye = GoalVerifierEye()

    @pytest.mark.asyncio
    async def test_llm_success(self):
        mock_response = MagicMock()
        mock_response.parsed = {
            "criteria_met": True,
            "confidence": 0.95,
            "reasoning": "Search form is visible and enabled",
            "criteria_status": "done",
            "sub_goal_advanced": True,
            "blocking_issues": [],
        }

        mock_client = MagicMock()
        mock_client.complete_structured = AsyncMock(return_value=mock_response)

        with patch("ans_nerves.eyes.goal_verifier.get_llm_client", return_value=mock_client):
            report = await self.eye.observe("s1", {
                "sub_goal_description": "Open search form",
                "success_criteria": ["Form is visible", "Input is enabled"],
                "page_url": "https://example.com",
            })
            assert report.confidence == 0.95
            assert report.content["criteria_met"] is True
            assert report.content["criteria_status"] == "done"

    @pytest.mark.asyncio
    async def test_llm_exception_falls_back(self):
        mock_client = MagicMock()
        mock_client.complete_structured = AsyncMock(side_effect=RuntimeError("API down"))

        with patch("ans_nerves.eyes.goal_verifier.get_llm_client", return_value=mock_client):
            report = await self.eye.observe("s1", {
                "sub_goal_description": "Open search form",
                "success_criteria": ["Form is visible"],
            })
            assert report.confidence == 0.0
            assert report.content["error"] == "LLM call failed"

    @pytest.mark.asyncio
    async def test_llm_none_parsed_falls_back(self):
        mock_response = MagicMock()
        mock_response.parsed = None
        mock_response.content = "garbled"

        mock_client = MagicMock()
        mock_client.complete_structured = AsyncMock(return_value=mock_response)

        with patch("ans_nerves.eyes.goal_verifier.get_llm_client", return_value=mock_client):
            report = await self.eye.observe("s1", {
                "sub_goal_description": "Open search form",
                "success_criteria": ["Form is visible"],
            })
            assert report.confidence == 0.0
            assert "raw_response" in report.content


class TestErrorDetectorEyeLLM:
    def setup_method(self):
        self.eye = ErrorDetectorEye()

    @pytest.mark.asyncio
    async def test_llm_success(self):
        mock_response = MagicMock()
        mock_response.parsed = {
            "failure_type": "timeout",
            "description": "Page took too long to load",
            "should_retry": True,
            "max_retries": 3,
            "recovery_actions": ["Wait and retry", "Check network"],
            "escalation_needed": False,
        }

        mock_client = MagicMock()
        mock_client.complete_structured = AsyncMock(return_value=mock_response)

        with patch("ans_nerves.eyes.error_detector.get_llm_client", return_value=mock_client):
            report = await self.eye.observe("s1", {
                "action_description": "Click search button",
                "error_message": "Timeout after 30s",
            })
            assert report.confidence == 0.85
            assert report.content["failure_type"] == "timeout"
            assert report.content["should_retry"] is True
            assert len(report.content["recovery_actions"]) == 2

    @pytest.mark.asyncio
    async def test_llm_exception_falls_back(self):
        mock_client = MagicMock()
        mock_client.complete_structured = AsyncMock(side_effect=RuntimeError("API down"))

        with patch("ans_nerves.eyes.error_detector.get_llm_client", return_value=mock_client):
            report = await self.eye.observe("s1", {
                "action_description": "Click search button",
                "error_message": "Timeout",
            })
            assert report.confidence == 0.3
            assert report.content["failure_type"] == "silent_fail"
            assert "LLM call failed" in report.content["description"]

    @pytest.mark.asyncio
    async def test_llm_none_parsed_falls_back(self):
        mock_response = MagicMock()
        mock_response.parsed = None
        mock_response.content = "garbled"

        mock_client = MagicMock()
        mock_client.complete_structured = AsyncMock(return_value=mock_response)

        with patch("ans_nerves.eyes.error_detector.get_llm_client", return_value=mock_client):
            report = await self.eye.observe("s1", {
                "action_description": "Click search button",
                "error_message": "Timeout",
            })
            assert report.confidence == 0.3
            assert "raw_response" in report.content
