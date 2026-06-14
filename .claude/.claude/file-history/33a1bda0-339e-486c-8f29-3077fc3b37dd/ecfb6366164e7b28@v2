"""Integration tests for the LLM client and prompt templates.

Set OPENAI_API_KEY to run the live tests. Without it, only unit tests
(mock client, prompt construction) will run.
"""

import json
import os
import sys

import pytest

from ans_nerves.config import get_config, NervesConfig
from ans_nerves.llm.prompts import (
    VISION_SYSTEM,
    VISION_JSON_SCHEMA,
    VERIFIER_SYSTEM,
    VERIFIER_JSON_SCHEMA,
    ERROR_DETECTOR_SYSTEM,
    ERROR_JSON_SCHEMA,
    DIFF_SYSTEM,
    DIFF_JSON_SCHEMA,
    DECOMPOSER_SYSTEM,
    DECOMPOSER_JSON_SCHEMA,
    COORDINATOR_SYSTEM,
    COORDINATOR_JSON_SCHEMA,
    build_vision_user_prompt,
    build_verifier_user_prompt,
    build_error_detector_user_prompt,
    build_diff_user_prompt,
    build_decomposer_user_prompt,
    build_coordinator_user_prompt,
)


# ── Unit tests: prompt construction ────────────────────────

class TestPromptConstruction:
    def test_vision_prompt_includes_all_sections(self):
        """Vision prompt is screenshot-only — no DOM context (cost optimization)."""
        prompt = build_vision_user_prompt(goal_context="Buy shoes", page_url="https://example.com")
        assert "<goal>" in prompt
        assert "Buy shoes" in prompt
        assert "<page_url>" in prompt
        assert "https://example.com" in prompt

    def test_verifier_prompt_includes_criteria(self):
        prompt = build_verifier_user_prompt(
            sub_goal_description="Fill search form",
            success_criteria=["input[aria-label='From'] value is Delhi"],
            page_url="https://example.com/search",
            page_title="Search Flights",
            visible_text=["From: Delhi", "To: Mumbai", "Search"],
        )
        assert "Fill search form" in prompt
        assert "input[aria-label='From']" in prompt
        assert "https://example.com/search" in prompt
        assert "From: Delhi" in prompt

    def test_error_detector_prompt_includes_action_and_error(self):
        prompt = build_error_detector_user_prompt(
            action_description="click button#search",
            error_message="Element not found: button#search",
            page_url="https://example.com",
            page_title="Error Page",
            visible_text=["404 Not Found"],
        )
        assert "click button#search" in prompt
        assert "Element not found" in prompt
        assert "404 Not Found" in prompt

    def test_diff_prompt_includes_urls(self):
        diff = json.dumps({"added": [], "removed": [], "modified": []})
        prompt = build_diff_user_prompt(
            diff,
            goal_context="Find flights",
            before_url="https://example.com/search",
            after_url="https://example.com/results",
        )
        assert "Find flights" in prompt
        assert "https://example.com/search" in prompt
        assert "https://example.com/results" in prompt

    def test_decomposer_prompt_includes_goal(self):
        prompt = build_decomposer_user_prompt(
            "Book the cheapest flight from Delhi to Mumbai on June 5",
            context={"budget_cents": 500, "max_steps": 50},
        )
        assert "Book the cheapest flight" in prompt
        assert "budget_cents" in prompt

    def test_coordinator_prompt_includes_reports(self):
        reports = json.dumps([
            {"eye_name": "vision", "content": {"page_type": "search_form"}},
            {"eye_name": "dom_reader", "content": {"elements": []}},
        ])
        prompt = build_coordinator_user_prompt(reports, goal_context="Search flights")
        assert "vision" in prompt
        assert "dom_reader" in prompt
        assert "Search flights" in prompt


# ── Schema tests: validate JSON schemas ────────────────────

class TestJsonSchemas:
    def test_vision_schema_has_required_fields(self):
        assert "page_type" in VISION_JSON_SCHEMA["required"]
        assert "visible_elements" in VISION_JSON_SCHEMA["required"]

    def test_verifier_schema_has_required_fields(self):
        assert "criteria_met" in VERIFIER_JSON_SCHEMA["required"]
        assert "confidence" in VERIFIER_JSON_SCHEMA["required"]
        assert "reasoning" in VERIFIER_JSON_SCHEMA["required"]

    def test_error_schema_has_valid_types(self):
        types = ERROR_JSON_SCHEMA["properties"]["failure_type"]["enum"]
        assert "silent_fail" in types
        assert "captcha" in types
        assert "paywall" in types

    def test_decomposer_schema_has_required_fields(self):
        assert "sub_goals" in DECOMPOSER_JSON_SCHEMA["required"]
        assert "estimated_steps" in DECOMPOSER_JSON_SCHEMA["required"]

    def test_coordinator_schema_handles_contradictions(self):
        assert "contradictions" in COORDINATOR_JSON_SCHEMA["properties"]
        assert "alerts" in COORDINATOR_JSON_SCHEMA["properties"]


# ── System prompt tests ────────────────────────────────────

class TestSystemPrompts:
    def test_vision_system_not_empty(self):
        assert len(VISION_SYSTEM) > 50
        assert "Vision Eye" in VISION_SYSTEM

    def test_verifier_system_has_rules(self):
        assert "conservative" in VERIFIER_SYSTEM.lower()

    def test_error_detector_has_all_types(self):
        assert "silent_fail" in ERROR_DETECTOR_SYSTEM
        assert "captcha" in ERROR_DETECTOR_SYSTEM

    def test_coordinator_has_resolution_hierarchy(self):
        assert "DOM wins" in COORDINATOR_SYSTEM
        assert "Vision wins" in COORDINATOR_SYSTEM

    def test_decomposer_system_has_rules(self):
        assert "SMALLEST" in DECOMPOSER_SYSTEM
        assert "success criterion" in DECOMPOSER_SYSTEM.lower()


# ── Integration tests (require OPENAI_API_KEY) ─────────────

@pytest.mark.skipif(
    not os.getenv("OPENAI_API_KEY"),
    reason="OPENAI_API_KEY not set — skipping live LLM tests",
)
class TestLiveLLM:
    """Live tests against GPT-4. Requires OPENAI_API_KEY env var."""

    @pytest.mark.asyncio
    async def test_client_returns_parsed_json(self):
        from ans_nerves.llm.client import get_llm_client

        client = get_llm_client()
        response = await client.complete(
            system_prompt="You are a helpful assistant. Respond with JSON.",
            user_prompt='Say hello. Use the format {"greeting": "your message"}.',
            json_mode=True,
            max_tokens=64,
        )
        assert response.parsed is not None
        assert "greeting" in response.parsed
        assert response.usage.total_tokens > 0
        assert response.latency_ms > 0

    @pytest.mark.asyncio
    async def test_vision_prompt_produces_valid_output(self):
        from ans_nerves.llm.client import get_llm_client

        client = get_llm_client()
        user_prompt = build_vision_user_prompt(
            goal_context="Find flights from Delhi to Mumbai",
            page_url="https://example.com/search",
        )

        response = await client.complete_structured(
            system_prompt=VISION_SYSTEM,
            user_prompt=user_prompt,
            json_schema=VISION_JSON_SCHEMA,
            max_tokens=512,
        )
        assert response.parsed is not None
        # Without an actual screenshot, this will hallucinate — but the schema should be valid
        assert "page_type" in response.parsed

    @pytest.mark.asyncio
    async def test_decomposer_produces_sub_goals(self):
        from ans_nerves.llm.client import get_llm_client

        client = get_llm_client()
        user_prompt = build_decomposer_user_prompt(
            "Search for 'mechanical keyboard' on Amazon and find the top-rated item under $100"
        )

        response = await client.complete_structured(
            system_prompt=DECOMPOSER_SYSTEM,
            user_prompt=user_prompt,
            json_schema=DECOMPOSER_JSON_SCHEMA,
            max_tokens=1024,
        )
        assert response.parsed is not None
        assert "sub_goals" in response.parsed
        assert len(response.parsed["sub_goals"]) >= 2
        # Each sub-goal should have success_criteria
        for sg in response.parsed["sub_goals"]:
            assert "success_criteria" in sg
            assert len(sg["success_criteria"]) >= 1

    @pytest.mark.asyncio
    async def test_verifier_returns_conservative_on_empty_evidence(self):
        from ans_nerves.llm.client import get_llm_client

        client = get_llm_client()
        user_prompt = build_verifier_user_prompt(
            sub_goal_description="Verify the search form is filled with 'Delhi' in the From field",
            success_criteria=["input#from value is 'Delhi'"],
            page_url="https://example.com/search",
            page_title="Search Flights",
            visible_text=["From: [empty]", "To: [empty]", "Search button"],
        )

        response = await client.complete_structured(
            system_prompt=VERIFIER_SYSTEM,
            user_prompt=user_prompt,
            json_schema=VERIFIER_JSON_SCHEMA,
            max_tokens=256,
        )
        assert response.parsed is not None
        # With empty evidence, should not be confident the criteria are met
        assert response.parsed.get("criteria_met") is False

    @pytest.mark.asyncio
    async def test_error_detector_classifies_404(self):
        from ans_nerves.llm.client import get_llm_client

        client = get_llm_client()
        user_prompt = build_error_detector_user_prompt(
            action_description="navigated to https://example.com/dead-page",
            error_message="Page returned HTTP 404",
            page_url="https://example.com/dead-page",
            page_title="404 Not Found",
            visible_text=["404", "Page not found", "The page you requested does not exist"],
        )

        response = await client.complete_structured(
            system_prompt=ERROR_DETECTOR_SYSTEM,
            user_prompt=user_prompt,
            json_schema=ERROR_JSON_SCHEMA,
            max_tokens=256,
        )
        assert response.parsed is not None
        assert response.parsed.get("failure_type") == "navigation_error"
        assert response.parsed.get("should_retry") is False
        assert len(response.parsed.get("recovery_actions", [])) >= 1

    @pytest.mark.skipif(
        sys.version_info >= (3, 14),
        reason="OpenAI SDK has known asyncio event-loop issues on Python 3.14",
    )
    @pytest.mark.asyncio
    async def test_client_tracks_token_usage(self):
        from ans_nerves.llm.client import get_llm_client

        client = get_llm_client()
        response = await client.complete(
            system_prompt="You are a counter. Respond with JSON.",
            user_prompt='Count to three. Format: {"count": 3}',
            json_mode=True,
            max_tokens=64,
        )
        assert response.usage.prompt_tokens > 0
        assert response.usage.completion_tokens > 0
        assert response.usage.total_tokens > 0
        assert response.usage.cost_cents > 0
