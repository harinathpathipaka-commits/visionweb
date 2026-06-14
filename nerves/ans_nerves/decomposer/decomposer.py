"""Goal Decomposer — LLM-powered goal → sub-goal decomposition.

Breaks high-level natural-language goals into verifiable SubGoal objects
with measurable success criteria that the Goal Verifier Eye can check.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from ans_nerves.config import get_config
from ans_nerves.llm.client import get_llm_client
from ans_nerves.logging import get_logger
from ans_nerves.llm.prompts import (
    DECOMPOSER_JSON_SCHEMA,
    DECOMPOSER_SYSTEM,
    build_decomposer_user_prompt,
)

logger = get_logger(__name__)

_DEFAULT_MAX_BUDGET_CENTS = 500
_DEFAULT_MAX_STEPS = 50


@dataclass
class SubGoal:
    """A single decomposable step with verifiable criteria."""

    id: str
    description: str
    success_criteria: list[str] = field(default_factory=list)
    depends_on: list[str] = field(default_factory=list)
    status: str = "pending"


@dataclass
class GoalSpec:
    """Full decomposition output."""

    description: str
    sub_goals: list[SubGoal] = field(default_factory=list)
    context: dict = field(default_factory=dict)
    max_budget_cents: int = _DEFAULT_MAX_BUDGET_CENTS
    max_steps: int = _DEFAULT_MAX_STEPS
    estimated_steps: int = 0
    risk_factors: list[str] = field(default_factory=list)


class GoalDecomposer:
    """Decomposes goals into verifiable sub-goals using LLM with response caching."""

    async def decompose(
        self,
        goal_description: str,
        context: dict | None = None,
    ) -> GoalSpec:
        """Break a goal into sub-goals using the LLM decomposer."""

        if not goal_description.strip():
            return GoalSpec(description=goal_description)

        user_prompt = build_decomposer_user_prompt(goal_description, context)

        try:
            response = await get_llm_client().complete_structured(
                system_prompt=DECOMPOSER_SYSTEM,
                user_prompt=user_prompt,
                json_schema=DECOMPOSER_JSON_SCHEMA,
                model_override=get_config().llm.decomposer_model,
            )
        except Exception:
            logger.warning("decomposer: LLM decomposition failed", exc_info=True)
            return GoalSpec(
                description=goal_description,
                context=context or {},
                risk_factors=["LLM decomposition failed"],
            )

        if response.parsed is None:
            logger.warning("decomposer: LLM returned unparseable response")
            return GoalSpec(
                description=goal_description,
                context=context or {},
                risk_factors=["LLM returned unparseable response"],
            )

        parsed = response.parsed
        sub_goals = [
            SubGoal(
                id=sg.get("id", f"sg_{i}"),
                description=sg.get("description", ""),
                success_criteria=sg.get("success_criteria", []),
                depends_on=sg.get("depends_on", []),
            )
            for i, sg in enumerate(parsed.get("sub_goals", []))
        ]

        spec = GoalSpec(
            description=goal_description,
            sub_goals=sub_goals,
            context=context or {},
            estimated_steps=parsed.get("estimated_steps", len(sub_goals)),
            risk_factors=parsed.get("risk_factors", []),
        )
        return spec

    async def decompose_single_sub_goal(
        self,
        sub_goal_description: str,
        full_goal: str,
        context: dict | None = None,
    ) -> list[SubGoal]:
        """Re-decompose a single stuck sub-goal into finer-grained steps.

        Called when a sub-goal is hitting max errors or stagnation — the
        original decomposition was too broad (e.g. "Fill the registration
        form" needs to become "Fill email field", "Fill password field", etc.).
        """
        if not sub_goal_description.strip():
            return []

        user_prompt = build_decomposer_user_prompt(
            goal_description=(
                f"RE-DECOMPOSE this specific sub-goal into SMALLER, more granular "
                f"steps. The sub-goal is part of a larger goal: '{full_goal}'.\n\n"
                f"SUB-GOAL TO BREAK DOWN: {sub_goal_description}\n\n"
                f"The original decomposition was too broad — the agent got stuck "
                f"because the step was too large. Break it into 2-4 VERY SPECIFIC "
                f"micro-steps that can each be completed with 1-3 browser actions."
            ),
            context=context or {},
        )

        try:
            response = await get_llm_client().complete_structured(
                system_prompt=DECOMPOSER_SYSTEM,
                user_prompt=user_prompt,
                json_schema=DECOMPOSER_JSON_SCHEMA,
                model_override=get_config().llm.decomposer_model,
            )
        except Exception:
            logger.warning("decomposer: re-decomposition LLM call failed")
            return []

        if response.parsed is None:
            return []

        return [
            SubGoal(
                id=sg.get("id", f"re_sg_{i}"),
                description=sg.get("description", ""),
                success_criteria=sg.get("success_criteria", []),
                depends_on=sg.get("depends_on", []),
            )
            for i, sg in enumerate(response.parsed.get("sub_goals", []))
        ]
