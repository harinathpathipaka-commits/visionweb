"""Cross-Eye Coordinator — LLM-powered synthesis of 5 eye reports.

Takes raw EyeReports from all 5 eyes, resolves contradictions via the
defined hierarchy, and produces a unified RoutedSignal for the Decision layer.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from typing import Any

from ans_nerves.eyes.base import EyeReport
from ans_nerves.llm.client import get_llm_client
from ans_nerves.llm.prompts import (
    COORDINATOR_JSON_SCHEMA,
    COORDINATOR_SYSTEM,
    build_coordinator_user_prompt,
)
from ans_nerves.logging import get_logger

logger = get_logger(__name__)

# Contradiction resolution hierarchy (matches Rust signal router)
_HIERARCHY = [
    "dom_reader",       # 1. DOM wins on element existence and interaction state
    "vision",            # 2. Vision wins on visual occlusion
    "page_diff",         # 3. Diff wins on whether the page actually changed
    "goal_verifier",     # 4. Goal Verifier wins on whether the goal advanced
    "error_detector",    # 5. Error Detector wins on whether an error occurred
]

_COORDINATOR_DEFAULTS: dict[str, Any] = {
    "unified_perception": "",
    "confidence": 0.0,
    "contradictions": [],
    "recommended_action_hint": None,
    "alerts": [],
}


@dataclass
class RoutedSignal:
    """Unified perception produced after synthesizing all eye reports."""

    unified_perception: str = ""
    alerts: list[dict] = field(default_factory=list)
    confidence: float = 1.0
    recommended_action_hint: str | None = None
    contradictions: list[dict] = field(default_factory=list)


class CrossEyeCoordinator:
    """Synthesizes all 5 eye reports into unified situational awareness.

    Uses GPT-4 to resolve contradictions and produce a natural-language
    summary. Falls back to deterministic merge when LLM is unavailable.
    """

    async def synthesize(
        self,
        reports: list[EyeReport],
        goal_context: str = "",
    ) -> RoutedSignal:
        """Combine all eye reports, resolve contradictions, produce unified signal."""
        if not reports:
            return RoutedSignal(
                unified_perception="No eye reports available.",
                confidence=0.0,
            )

        # Build a JSON summary of all reports for the LLM
        reports_json = json.dumps([
            {
                "eye_name": r.eye_name,
                "confidence": r.confidence,
                "goal_relevance": r.goal_relevance,
                "content": r.content,
            }
            for r in reports
        ])

        user_prompt = build_coordinator_user_prompt(
            eye_reports_json=reports_json,
            goal_context=goal_context,
        )

        try:
            response = await get_llm_client().complete_structured(
                system_prompt=COORDINATOR_SYSTEM,
                user_prompt=user_prompt,
                json_schema=COORDINATOR_JSON_SCHEMA,
            )
        except Exception:
            logger.warning("coordinator: LLM synthesis failed, using fallback")
            return self._fallback_synthesize(reports)

        if response.parsed is None:
            logger.warning("coordinator: LLM returned unparseable JSON, using fallback")
            return self._fallback_synthesize(reports)

        return RoutedSignal(
            unified_perception=response.parsed.get("unified_perception", ""),
            confidence=response.parsed.get("confidence", 0.5),
            recommended_action_hint=response.parsed.get("recommended_action_hint"),
            contradictions=response.parsed.get("contradictions", []),
            alerts=response.parsed.get("alerts", []),
        )

    def _fallback_synthesize(
        self, reports: list[EyeReport], diff_data: dict | None = None,
    ) -> RoutedSignal:
        """Deterministic synthesis using the contradiction hierarchy.

        No LLM — purely structural merge. Includes raw diff summary from
        the Rust diff engine when available, so PageDiff LLM is unnecessary.
        """
        perception_parts: list[str] = []
        highest_confidence = 0.0
        total_relevance = 0.0

        for r in reports:
            if r.confidence > highest_confidence:
                highest_confidence = r.confidence
            total_relevance += r.goal_relevance

        # Summarise each eye's key findings
        for r in reports:
            content = r.content
            if r.eye_name == "dom_reader":
                interactive = content.get("interactive", [])
                noise = len(content.get("distraction_flags", []))
                # Summarise form fields with labels for the planner
                form_fields = []
                for el in interactive:
                    label = el.get("label", "") or el.get("placeholder", "") or el.get("text", "")
                    tag = el.get("element_type", el.get("tag", ""))
                    itype = el.get("input_type", "")
                    name = el.get("name", "")
                    if tag in ("input", "textarea", "select", "button"):
                        desc = f"tag={tag}"
                        if itype and itype != "text":
                            desc += f" type={itype}"
                        if name:
                            desc += f" name={name}"
                        if label:
                            desc += f" label='{label[:40]}'"
                        form_fields.append(desc)
                form_summary = ""
                if form_fields:
                    form_summary = f" fields=[{', '.join(form_fields[:12])}]"
                perception_parts.append(
                    f"DOM: {len(interactive)} interactive, {noise} noise{form_summary}"
                )
            elif r.eye_name == "vision":
                page_type = content.get("page_type", "unknown")
                overlays = len(content.get("overlays", []))
                anomalies = content.get("anomalies", [])
                perception_parts.append(
                    f"Vision: page_type={page_type}, overlays={overlays}"
                )
                if anomalies:
                    perception_parts.append(f"Anomalies: {', '.join(anomalies)}")
            elif r.eye_name == "page_diff":
                summary = content.get("summary", "no_change")
                perception_parts.append(f"Diff: {summary}")
            elif r.eye_name == "goal_verifier":
                met = content.get("criteria_met", False)
                perception_parts.append(
                    f"Verification: criteria_met={met}"
                )
            elif r.eye_name == "error_detector":
                failure = content.get("failure_type", "none")
                desc = content.get("description", "")
                if failure in ("none", "silent_fail") and not desc:
                    pass
                elif desc:
                    perception_parts.append(f"Error: {failure}: {desc}")
                else:
                    perception_parts.append(f"Error: {failure}")

        # Include structural diff summary from Rust engine (no LLM needed)
        if diff_data:
            added = diff_data.get("added", 0)
            removed = diff_data.get("removed", 0)
            summary = diff_data.get("summary", "")
            if isinstance(added, list):
                added = len(added)
            if isinstance(removed, list):
                removed = len(removed)
            if added or removed or summary:
                perception_parts.append(
                    f"Diff: +{added}/-{removed} elements changed"
                )
                if summary:
                    perception_parts[-1] += f" ({summary})"

        avg_relevance = total_relevance / len(reports) if reports else 0.0

        return RoutedSignal(
            unified_perception=" | ".join(perception_parts),
            confidence=highest_confidence,
            recommended_action_hint=None,
            contradictions=[],
            alerts=[],
        )
