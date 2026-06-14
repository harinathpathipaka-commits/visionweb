"""DOM Reader Eye — structured DOM perception without LLM.

Consumes DistilledDom from gRPC, classifies elements into interactive,
semantic, and noise categories. Pure Python — no LLM call needed.
"""

from __future__ import annotations

from typing import Any

from ans_nerves.eyes.base import BaseEye, EyeReport


_INTERACTIVE_TAGS = frozenset({
    "a", "button", "input", "select", "textarea", "option",
    "form", "label", "fieldset", "datalist", "output",
})

_SEMANTIC_TAGS = frozenset({
    "h1", "h2", "h3", "h4", "h5", "h6", "p", "article",
    "section", "nav", "header", "footer", "main", "aside",
    "table", "ul", "ol", "li", "dl", "dt", "dd",
    "blockquote", "pre", "code", "figure", "figcaption",
})

_NOISE_CLASS_PATTERNS = frozenset({
    "ad", "ads", "advert", "advertisement", "banner",
    "cookie", "consent", "gdpr", "popup", "modal",
    "newsletter", "subscribe", "social", "share",
    "tracking", "tracker", "analytics",
})


def _is_noise(element: dict[str, Any]) -> bool:
    """Classify an element as noise based on CSS classes, IDs, and ARIA roles."""
    css_class = (element.get("css_class") or "").lower()
    css_id = (element.get("css_id") or "").lower()
    role = (element.get("aria_role") or "").lower()

    combined = f"{css_class} {css_id} {role}"
    for pattern in _NOISE_CLASS_PATTERNS:
        if pattern in combined:
            return True
    return False


def _classify_element(element: dict[str, Any]) -> str:
    """Classify a single element as interactive, semantic, or noise."""
    if _is_noise(element):
        return "noise"

    tag = (element.get("tag") or "").lower()
    if tag in _INTERACTIVE_TAGS:
        return "interactive"
    if tag in _SEMANTIC_TAGS:
        return "semantic"

    # Check ARIA roles for interactive hints
    role = (element.get("aria_role") or "").lower()
    if role in {"button", "link", "textbox", "searchbox", "combobox",
                 "checkbox", "radio", "switch", "menuitem", "option", "tab"}:
        return "interactive"

    return "semantic"


class DomReaderEye(BaseEye):
    """Reads distilled DOM and produces structured element lists.

    No LLM call — pure heuristic classification runs in microseconds.
    """

    name = "dom_reader"

    async def observe(self, session_id: str, page_data: dict[str, Any]) -> EyeReport:
        distilled = page_data.get("distilled_dom")
        if distilled is None:
            return EyeReport(
                eye_name=self.name,
                confidence=0.0,
                content={
                    "elements": [],
                    "interactive": [],
                    "semantic_blocks": [],
                    "distraction_flags": [],
                    "mode": "text_only",
                    "error": "No distilled DOM in page_data",
                },
            )

        # Accept both already-parsed dict and JSON string
        if isinstance(distilled, str):
            import json
            try:
                distilled = json.loads(distilled)
            except json.JSONDecodeError:
                return EyeReport(
                    eye_name=self.name,
                    confidence=0.0,
                    content={"error": "Invalid distilled DOM JSON"},
                )

        elements: list[dict[str, Any]] = distilled.get("elements", [])
        mode = distilled.get("mode", "text_only")

        interactive = []
        semantic_blocks = []
        distraction_flags = []

        for el in elements:
            classification = _classify_element(el)

            if classification == "interactive":
                attrs = el.get("attributes", {}) or {}
                interactive.append({
                    "selector": el.get("selector", ""),
                    "element_index": el.get("element_index"),
                    "element_type": el.get("tag", "unknown"),
                    "label": el.get("text", "") or el.get("aria_label", "") or "",
                    "input_type": attrs.get("type", "text") if el.get("tag") == "input" else None,
                    "placeholder": attrs.get("placeholder", ""),
                    "name": attrs.get("name", ""),
                    "autocomplete": attrs.get("autocomplete", ""),
                    "aria_label": el.get("aria_label", ""),
                    "value": attrs.get("value", ""),
                    "is_visible": el.get("is_visible", True),
                    "is_enabled": el.get("is_enabled", True),
                    "bounding_box": el.get("bounding_box"),
                    "goal_relevance_score": el.get("goal_relevance_score", 0.0),
                })
            elif classification == "noise":
                distraction_flags.append({
                    "selector": el.get("selector", ""),
                    "kind": "classified_by_css",
                    "confidence": 0.85,
                    "text_snippet": (el.get("text") or "")[:100],
                })
            else:
                semantic_blocks.append({
                    "block_type": el.get("tag", "unknown"),
                    "text_content": el.get("text", "") or "",
                    "goal_relevance_score": el.get("goal_relevance_score", 0.0),
                })

        return EyeReport(
            eye_name=self.name,
            confidence=1.0,
            goal_relevance=self._compute_overall_relevance(interactive, semantic_blocks),
            content={
                "elements": elements,
                "interactive": interactive,
                "semantic_blocks": semantic_blocks,
                "distraction_flags": distraction_flags,
                "mode": mode,
                "url": distilled.get("url", ""),
                "title": distilled.get("title", ""),
            },
        )

    @staticmethod
    def _compute_overall_relevance(
        interactive: list[dict[str, Any]],
        semantic_blocks: list[dict[str, Any]],
    ) -> float:
        scores: list[float] = []
        for el in interactive:
            s = el.get("goal_relevance_score", 0.0)
            if isinstance(s, (int, float)):
                scores.append(float(s))
        for blk in semantic_blocks:
            s = blk.get("goal_relevance_score", 0.0)
            if isinstance(s, (int, float)):
                scores.append(float(s))

        if not scores:
            return 0.0
        return sum(scores) / len(scores)
