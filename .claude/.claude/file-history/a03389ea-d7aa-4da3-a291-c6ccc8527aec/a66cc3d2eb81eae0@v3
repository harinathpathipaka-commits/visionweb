"""Page Diff Eye — semantic interpretation of page changes via GPT-4.

Wraps the Rust diff engine output. Enriches structural diffs with
semantic interpretation: "search results updated", "form field filled".
"""

from __future__ import annotations

import json
from typing import Any

from ans_nerves.eyes.base import BaseEye, EyeReport
from ans_nerves.llm.client import get_llm_client
from ans_nerves.logging import get_logger
from ans_nerves.llm.prompts import (
    DIFF_JSON_SCHEMA,
    DIFF_SYSTEM,
    build_diff_user_prompt,
)

logger = get_logger(__name__)

_DIFF_DEFAULTS: dict[str, Any] = {
    "summary": "no_change",
    "what_changed": "",
    "is_goal_relevant": False,
    "new_elements_of_interest": [],
    "confidence": 1.0,
}


class PageDiffEye(BaseEye):
    """Detects and interprets page changes using GPT-4."""

    name = "page_diff"

    async def observe(self, session_id: str, page_data: dict[str, Any]) -> EyeReport:
        diff = page_data.get("diff")
        if diff is None:
            return EyeReport(
                eye_name=self.name,
                confidence=1.0,
                content={
                    **_DIFF_DEFAULTS,
                    "summary": "no_change",
                    "what_changed": "No diff data provided",
                },
            )

        if isinstance(diff, dict):
            diff = json.dumps(diff)

        goal_context = page_data.get("goal_context", "")
        before_url = page_data.get("before_url", "")
        after_url = page_data.get("after_url", page_data.get("url", ""))

        user_prompt = build_diff_user_prompt(
            diff_json=diff,
            goal_context=goal_context,
            before_url=before_url,
            after_url=after_url,
        )

        try:
            response = await get_llm_client().complete_structured(
                system_prompt=DIFF_SYSTEM,
                user_prompt=user_prompt,
                json_schema=DIFF_JSON_SCHEMA,
            )
        except Exception:
            logger.warning("page_diff: LLM call failed", exc_info=True)
            return EyeReport(
                eye_name=self.name,
                confidence=0.3,
                content={**_DIFF_DEFAULTS, "error": "LLM call failed"},
            )

        if response.parsed is None:
            logger.warning("page_diff: LLM returned unparseable response")
            return EyeReport(
                eye_name=self.name,
                confidence=0.3,
                content={**_DIFF_DEFAULTS, "raw_response": response.content[:500]},
            )

        return EyeReport(
            eye_name=self.name,
            confidence=response.parsed.get("confidence", 0.8),
            content=response.parsed,
        )
