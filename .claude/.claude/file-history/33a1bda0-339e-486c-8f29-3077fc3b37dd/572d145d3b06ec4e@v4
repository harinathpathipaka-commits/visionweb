"""Prompt templates for each Eye and the Goal Decomposer.

Every Eye gets a dedicated system prompt and a helper to build the
user prompt from page state / goal context. Templates stay here so
they can be iterated independently of the eye implementations.
"""

from __future__ import annotations

import json
from typing import Any

# ─────────────────────────────────────────────────────────────
# Vision Eye — screenshot + DOM → structured visual report
# ─────────────────────────────────────────────────────────────

VISION_SYSTEM = """You are the Vision Eye of an autonomous browser agent. Your job is to analyse a screenshot and distilled DOM of a web page and produce a structured report of what is visible.

You are NOT the agent making decisions. You are a sensory organ — observe and report facts, nothing more.

Rules:
1. List only elements that are ACTUALLY visible in the screenshot — not just present in the DOM.
2. Flag any overlays, popups, cookie banners, or modals blocking the main content.
3. Identify the page type (search_form, search_results, product_page, checkout, login, error, article, dashboard, captcha, paywall, unknown).
4. Note any anomalies: broken layouts, missing images, error messages, unexpected redirects.
5. Provide bounding box estimates for key interactive elements."""

VISION_JSON_SCHEMA = {
    "type": "object",
    "properties": {
        "page_type": {
            "type": "string",
            "enum": [
                "search_form", "search_results", "product_page", "checkout",
                "login", "error", "article", "dashboard", "captcha",
                "paywall", "form", "unknown",
            ],
            "description": "The type of page shown.",
        },
        "visible_elements": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "selector": {"type": "string"},
                    "element_type": {"type": "string"},
                    "label": {"type": "string"},
                    "is_visible": {"type": "boolean"},
                    "is_enabled": {"type": "boolean"},
                    "is_blocked": {"type": "boolean"},
                    "bounding_box": {
                        "type": "object",
                        "properties": {
                            "x": {"type": "number"},
                            "y": {"type": "number"},
                            "width": {"type": "number"},
                            "height": {"type": "number"},
                        },
                    },
                },
                "required": ["selector", "element_type", "label", "is_visible"],
            },
            "description": "Key interactive and semantic elements visible on screen.",
        },
        "overlays": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "type": {"type": "string"},
                    "description": {"type": "string"},
                    "blocks_content": {"type": "boolean"},
                },
            },
            "description": "Popups, modals, cookie banners, or other overlays.",
        },
        "blocked_regions": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "description": {"type": "string"},
                    "bounding_box": {
                        "type": "object",
                        "properties": {
                            "x": {"type": "number"},
                            "y": {"type": "number"},
                            "width": {"type": "number"},
                            "height": {"type": "number"},
                        },
                    },
                },
            },
            "description": "Regions of the page occluded by overlays or rendering issues.",
        },
        "anomalies": {
            "type": "array",
            "items": {"type": "string"},
            "description": "Broken layouts, error messages, missing content, unexpected states.",
        },
    },
    "required": ["page_type", "visible_elements", "overlays", "anomalies"],
}


def build_vision_user_prompt(
    goal_context: str = "",
    page_url: str = "",
) -> str:
    """Build a screenshot-only Vision prompt — no DOM context.

    Vision's job is to observe what's in the screenshot, not to correlate
    DOM elements. DOM Reader handles structure. Separating concerns avoids
    hallucinated selectors and saves ~3000 input tokens per call.
    """
    parts = []
    if goal_context:
        parts.append(f"<goal>\n{goal_context}\n</goal>")
    if page_url:
        parts.append(f"<page_url>\n{page_url}\n</page_url>")
    parts.append(
        "Analyse the screenshot of this page (attached as an image). "
        "Report what is visible, what is blocked, what type of page this is, "
        "and any anomalies present."
    )
    return "\n\n".join(parts)


# ─────────────────────────────────────────────────────────────
# Goal Verifier Eye — page state + criteria → verification
# ─────────────────────────────────────────────────────────────

VERIFIER_SYSTEM = """You are the Goal Verifier Eye of an autonomous browser agent. Your job is to check whether a specific sub-goal or criterion has been met, given the current page state.

You are NOT the agent. Your only job is to verify, factually and conservatively.

Rules:
1. Only mark a criterion as met if there is CLEAR evidence in the provided page state.
2. If you are unsure, set confidence low and criteria_met to false.
3. Use the page URL, visible text, and DOM element states as evidence.
4. If the evidence is ambiguous, explain why in the reasoning field."""

VERIFIER_JSON_SCHEMA = {
    "type": "object",
    "properties": {
        "criteria_met": {
            "type": "boolean",
            "description": "Whether ALL criteria for this sub-goal are satisfied.",
        },
        "confidence": {
            "type": "number",
            "description": "Confidence 0.0-1.0 in this verdict.",
        },
        "criteria_status": {
            "type": "object",
            "description": "Per-criterion status with evidence.",
            "additionalProperties": {
                "type": "object",
                "properties": {
                    "met": {"type": "boolean"},
                    "evidence": {"type": "string"},
                },
            },
        },
        "sub_goal_advanced": {
            "type": "boolean",
            "description": "True if any progress was made toward the sub-goal, even if not complete.",
        },
        "reasoning": {
            "type": "string",
            "description": "Step-by-step reasoning. What was checked and what was found.",
        },
        "blocking_issues": {
            "type": "array",
            "items": {"type": "string"},
            "description": "Anything preventing the sub-goal from completing.",
        },
    },
    "required": ["criteria_met", "confidence", "reasoning"],
}


def build_verifier_user_prompt(
    sub_goal_description: str,
    success_criteria: list[str],
    page_url: str,
    page_title: str,
    visible_text: list[str],
    dom_summary: str = "",
    diff_summary: str = "",
) -> str:
    criteria_block = "\n".join(f"  {i+1}. {c}" for i, c in enumerate(success_criteria))
    text_block = "\n".join(visible_text[:100])  # cap at 100 lines

    parts = [
        f"<sub_goal>\n{sub_goal_description}\n</sub_goal>",
        f"<success_criteria>\n{criteria_block}\n</success_criteria>",
        f"<page>\nURL: {page_url}\nTitle: {page_title}\n</page>",
        f"<visible_text>\n{text_block}\n</visible_text>",
    ]
    if dom_summary:
        parts.append(f"<dom_summary>\n{dom_summary}\n</dom_summary>")
    if diff_summary:
        parts.append(f"<diff_summary>\n{diff_summary}\n</diff_summary>")

    # Truncate DOM summary to reduce input tokens
    if dom_summary:
        dom_summary = dom_summary[:2000]
    parts.append(
        "Verify whether the success criteria are met given the current page state. "
        "Be conservative — only mark criteria as met with clear evidence."
    )
    return "\n\n".join(parts)


# ─────────────────────────────────────────────────────────────
# Error Detector Eye — page state → failure classification
# ─────────────────────────────────────────────────────────────

ERROR_DETECTOR_SYSTEM = """You are the Error Detector Eye of an autonomous browser agent. Your job is to classify what went wrong when an action fails or the page state is unexpected.

You are NOT the agent. Classify the failure and suggest recovery actions.

Failure types:
- silent_fail: Action executed but had no visible effect. Page unchanged.
- wrong_element: Clicked/typed the wrong element. Expected element not found.
- blocked_interaction: Element exists but is blocked by an overlay, disabled, or hidden.
- state_mismatch: Page state doesn't match what was expected. Wrong page loaded.
- goal_drift: Action succeeded technically but didn't advance the goal.
- timeout: Page took too long to load or element didn't become interactive.
- navigation_error: Navigation failed — wrong URL, 404, 500, DNS error.
- captcha: A CAPTCHA or bot-detection challenge is blocking progress.
- paywall: Content is behind a paywall.
- login_wall: Login required to proceed."""

ERROR_JSON_SCHEMA = {
    "type": "object",
    "properties": {
        "failure_type": {
            "type": "string",
            "enum": [
                "silent_fail", "wrong_element", "blocked_interaction",
                "state_mismatch", "goal_drift", "timeout",
                "navigation_error", "captcha", "paywall", "login_wall",
            ],
        },
        "description": {
            "type": "string",
            "description": "What went wrong in one sentence.",
        },
        "should_retry": {
            "type": "boolean",
            "description": "Whether retrying the same action could help.",
        },
        "max_retries": {
            "type": "integer",
            "description": "Suggested max retry count (1-3).",
        },
        "recovery_actions": {
            "type": "array",
            "items": {"type": "string"},
            "description": "Ordered recovery steps to try.",
        },
        "escalation_needed": {
            "type": "boolean",
            "description": "True if human intervention or goal re-planning is needed.",
        },
    },
    "required": ["failure_type", "description", "should_retry", "recovery_actions"],
}


def build_error_detector_user_prompt(
    action_description: str,
    error_message: str,
    page_url: str,
    page_title: str,
    visible_text: list[str],
    goal_context: str = "",
) -> str:
    text_block = "\n".join(visible_text[:50])
    parts = [
        f"<failed_action>\n{action_description}\n</failed_action>",
        f"<error>\n{error_message}\n</error>",
        f"<page>\nURL: {page_url}\nTitle: {page_title}\n</page>",
        f"<visible_text>\n{text_block}\n</visible_text>",
    ]
    if goal_context:
        parts.append(f"<goal>\n{goal_context}\n</goal>")
    parts.append(
        "Classify this failure and suggest recovery actions. "
        "Be specific — name exact selectors if you can identify them."
    )
    return "\n\n".join(parts)


# ─────────────────────────────────────────────────────────────
# Page Diff Eye — diff delta → semantic interpretation
# ─────────────────────────────────────────────────────────────

DIFF_SYSTEM = """You are the Page Diff Eye of an autonomous browser agent. Your job is to interpret what changed between two page states and whether the change is meaningful for the current goal.

You are NOT the agent. Interpret structural diffs into semantic meaning.

Summary types:
- no_change: Nothing meaningful changed (cosmetic only).
- cosmetic_change: Animations, timestamps, minor text updates.
- content_update: New content loaded (search results, dynamic content).
- form_update: Input field values changed.
- navigation: A new page loaded.
- distraction_appeared: A popup, banner, or modal appeared and needs dismissal.
- error_state: An error message or broken state appeared."""

DIFF_JSON_SCHEMA = {
    "type": "object",
    "properties": {
        "summary": {
            "type": "string",
            "enum": [
                "no_change", "cosmetic_change", "content_update",
                "form_update", "navigation", "distraction_appeared",
                "error_state",
            ],
        },
        "what_changed": {
            "type": "string",
            "description": "Human-readable description of what changed.",
        },
        "is_goal_relevant": {
            "type": "boolean",
            "description": "Whether this change advances or blocks the goal.",
        },
        "new_elements_of_interest": {
            "type": "array",
            "items": {"type": "string"},
            "description": "Selectors or descriptions of new elements worth interacting with.",
        },
        "confidence": {
            "type": "number",
            "description": "Confidence in this interpretation 0.0-1.0.",
        },
    },
    "required": ["summary", "what_changed", "is_goal_relevant"],
}


def build_diff_user_prompt(
    diff_json: str,
    goal_context: str = "",
    before_url: str = "",
    after_url: str = "",
) -> str:
    parts = []
    if goal_context:
        parts.append(f"<goal>\n{goal_context}\n</goal>")
    parts.append(f"<before_url>\n{before_url}\n</before_url>")
    parts.append(f"<after_url>\n{after_url}\n</after_url>")
    parts.append(f"<diff>\n{diff_json}\n</diff>")
    parts.append(
        "Interpret this diff. What changed semantically? "
        "Is it relevant to the goal? What should the agent pay attention to?"
    )
    return "\n\n".join(parts)


# ─────────────────────────────────────────────────────────────
# Goal Decomposer — goal text → sub-goals with criteria
# ─────────────────────────────────────────────────────────────

DECOMPOSER_SYSTEM = """You are the Goal Decomposer of an autonomous browser agent. Your job is to break a high-level goal into a sequence of verifiable sub-goals, each with measurable success criteria.

You are NOT the agent executing the goal. Your output drives the verification loop.

Rules:
1. Break the goal into the SMALLEST meaningful steps. A sub-goal should be one action or one verification.
2. Each sub-goal MUST have at least one success criterion that can be checked from page state (URL, DOM, visible text).
3. Order sub-goals logically. Mark dependencies where step B cannot start before step A completes.
4. Be specific: "Fill the From field with 'Delhi'" not "Fill the form".
5. Include error recovery sub-goals for known failure modes (e.g., "If CAPTCHA appears, stop and report")."""

DECOMPOSER_JSON_SCHEMA = {
    "type": "object",
    "properties": {
        "sub_goals": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "description": {"type": "string"},
                    "success_criteria": {
                        "type": "array",
                        "items": {"type": "string"},
                    },
                    "depends_on": {
                        "type": "array",
                        "items": {"type": "string"},
                    },
                    "expected_url_pattern": {"type": "string"},
                },
                "required": ["id", "description", "success_criteria"],
            },
        },
        "estimated_steps": {"type": "integer"},
        "risk_factors": {
            "type": "array",
            "items": {"type": "string"},
            "description": "Things that could go wrong (captchas, logins, paywalls, etc.).",
        },
    },
    "required": ["sub_goals", "estimated_steps"],
}


def build_decomposer_user_prompt(
    goal_description: str,
    context: dict[str, Any] | None = None,
) -> str:
    parts = [f"<goal>\n{goal_description}\n</goal>"]
    if context:
        parts.append(
            "<context>\n" + json.dumps(context, indent=2) + "\n</context>"
        )
    parts.append(
        "Decompose this goal into sub-goals with verifiable success criteria. "
        "Each sub-goal should be small enough that success can be verified "
        "by checking the page URL, DOM elements, or visible text."
    )
    return "\n\n".join(parts)


# ─────────────────────────────────────────────────────────────
# Coordinator — 5 eye reports → unified RoutedSignal
# ─────────────────────────────────────────────────────────────

COORDINATOR_SYSTEM = """You are the Cross-Eye Coordinator of an autonomous browser agent. Your job is to synthesise reports from 5 sensory eyes into a single unified perception, resolving any contradictions.

You are NOT the agent. Synthesise and route — do not decide actions.

Contradiction resolution hierarchy (when two eyes disagree):
1. DOM wins on element existence and interaction state (is_visible, is_enabled).
2. Vision wins on visual occlusion (overlays, blocked regions).
3. Diff wins on whether the page actually changed.
4. Goal Verifier wins on whether the goal advanced.
5. Error Detector wins on whether an error occurred.
6. Injection Detector wins on dangerous content detection."""

COORDINATOR_JSON_SCHEMA = {
    "type": "object",
    "properties": {
        "unified_perception": {
            "type": "string",
            "description": "Concise natural-language summary of the current situation. What page we're on, what's visible, what changed, whether we're on track.",
        },
        "confidence": {
            "type": "number",
            "description": "Overall confidence in the unified perception 0.0-1.0.",
        },
        "contradictions": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "eyes_involved": {"type": "array", "items": {"type": "string"}},
                    "issue": {"type": "string"},
                    "resolution": {"type": "string"},
                    "winner": {"type": "string"},
                },
            },
            "description": "Any contradictions found and how they were resolved.",
        },
        "recommended_action_hint": {
            "type": "string",
            "description": "A hint for the decision layer about what action makes sense next. Null if unclear.",
        },
        "alerts": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "severity": {"type": "string", "enum": ["info", "warning", "critical"]},
                    "message": {"type": "string"},
                },
            },
        },
    },
    "required": ["unified_perception", "confidence"],
}


def build_coordinator_user_prompt(
    eye_reports_json: str,
    goal_context: str = "",
) -> str:
    parts = []
    if goal_context:
        parts.append(f"<goal>\n{goal_context}\n</goal>")
    parts.append(f"<eye_reports>\n{eye_reports_json}\n</eye_reports>")
    parts.append(
        "Synthesise all eye reports into a unified situational perception. "
        "Identify and resolve any contradictions between eyes. "
        "Provide a clear, concise summary of the current state."
    )
    return "\n\n".join(parts)


# ─────────────────────────────────────────────────────────────
# Agent Planner — context → next action (cold start + warm start)
# ─────────────────────────────────────────────────────────────

PLANNER_SYSTEM = """You are the Action Planner of an autonomous browser agent. Your job is to decide the SINGLE next browser action to take, given the current page state, goal context, and action history.

You are the DECISION MAKER. Output exactly one action that brings the agent closer to its goal.

Rules:
1. Choose the most impactful single action. One click, one fill, one navigation.
2. Use specific CSS selectors when you can identify them from the page state.
3. Prefer interacting with interactive elements (buttons, inputs, links, selects) over passive observation.
4. If an overlay/popup/cookie-banner blocks the page, dismiss it FIRST.
5. If the goal is complete (all sub-goals verified), output action_type="done".
6. If you're stuck or unsure, output action_type="wait" with a reasoning that helps diagnose.
7. When memory recommendations are provided, use them as STRONG hints — but override if the current page state contradicts them.

Action types you can output:
- click: Click a specific element by CSS selector
- type: Type text into an input field (use value= for the text)
- select: Choose an option from a <select> element
- navigate: Go to a URL
- scroll: Scroll the page
- wait: Wait for something to load/change
- dismiss_overlay: Close a popup, modal, or cookie banner
- evaluate: Execute JavaScript in the page and return the result
- done: The goal/sub-goal is complete — no further actions needed
- escalate: Human intervention needed (captcha, paywall, unexpected error)"""

PLANNER_JSON_SCHEMA = {
    "type": "object",
    "properties": {
        "action_type": {
            "type": "string",
            "enum": [
                "click", "type", "select", "navigate", "scroll",
                "wait", "dismiss_overlay", "evaluate", "done", "escalate",
            ],
            "description": "The type of browser action to take.",
        },
        "selector": {
            "type": "string",
            "description": "CSS selector of the target element. Empty for navigate/scroll/wait/done/escalate.",
        },
        "value": {
            "type": "string",
            "description": "Value to type for 'type', URL for 'navigate', option text for 'select'. Empty otherwise.",
        },
        "tool": {
            "type": "string",
            "enum": ["click", "type", "select", "navigate", "scroll", "wait", "dismiss_overlay", "evaluate", "done", "escalate"],
            "description": "The gRPC tool to call. Must match action_type.",
        },
        "reasoning": {
            "type": "string",
            "description": "Step-by-step reasoning: what you see, why this action, what you expect to happen.",
        },
        "confidence": {
            "type": "number",
            "description": "Confidence 0.0-1.0 that this is the right next action.",
        },
        "expected_outcome": {
            "type": "string",
            "description": "What you expect to change on the page after this action.",
        },
    },
    "required": ["action_type", "selector", "value", "tool", "reasoning", "confidence"],
}


def build_planner_user_prompt(
    goal_context: str,
    sub_goal: str = "",
    sub_goal_criteria: list[str] | None = None,
    unified_perception: str = "",
    available_elements: list[dict] | None = None,
    action_history: list[str] | None = None,
    memory_recommendations: list[dict] | None = None,
    last_error: str = "",
) -> str:
    """Build the planner user prompt with full situational context.

    Cold start: memory_recommendations is empty → LLM reasons from scratch.
    Warm start: memory_recommendations has scored past actions → LLM validates.
    """
    parts = []

    # Goal
    parts.append(f"<goal>\n{goal_context}\n</goal>")
    if sub_goal:
        parts.append(f"<current_sub_goal>\n{sub_goal}\n</current_sub_goal>")
    if sub_goal_criteria:
        criteria_block = "\n".join(f"  - {c}" for c in sub_goal_criteria)
        parts.append(f"<success_criteria>\n{criteria_block}\n</success_criteria>")

    # Current perception
    if unified_perception:
        parts.append(f"<page_state>\n{unified_perception}\n</page_state>")

    # Available interactive elements
    if available_elements:
        elems_block = json.dumps(available_elements, indent=2)
        parts.append(f"<interactive_elements>\n{elems_block}\n</interactive_elements>")

    # Action history
    if action_history:
        history_block = "\n".join(f"  {i+1}. {a}" for i, a in enumerate(action_history[-10:]))
        parts.append(f"<recent_actions>\n{history_block}\n</recent_actions>")

    # Last error
    if last_error:
        parts.append(f"<last_error>\n{last_error}\n</last_error>")

    # Memory recommendations (warm start)
    if memory_recommendations:
        recs_block = json.dumps(memory_recommendations, indent=2)
        parts.append(
            f"<memory_recommendations>\n"
            f"The following actions worked well in similar past contexts. "
            f"Use them as strong hints, but verify they make sense for the CURRENT page state:\n"
            f"{recs_block}\n</memory_recommendations>"
        )

    # Decision prompt
    if memory_recommendations:
        parts.append(
            "Based on the goal, current page state, and memory recommendations above, "
            "decide the SINGLE next browser action. Validate that memory recommendations "
            "match the current page — override if they don't."
        )
    else:
        parts.append(
            "Based on the goal and current page state, reason step-by-step and decide "
            "the SINGLE next browser action. No memory is available yet, so reason purely "
            "from context — what happened, where we are, and what should happen next."
        )

    return "\n\n".join(parts)
