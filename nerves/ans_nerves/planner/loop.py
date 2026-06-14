"""Agent Execution Loop — full perception→decision→action→learning cycle.

Orchestrates:
1. Decompose goal → sub_goals
2. For each sub_goal:
   a. Perceive (all 5 Eyes)
   b. Synthesize (CrossEyeCoordinator)
   c. Decide (AgentPlanner — cold or warm start)
   d. Execute (gRPC client)
   e. Learn (DecisionIntelligence — score → embed → store)
   f. Verify (GoalVerifier)
3. Re-plan on failure, escalate on deadlock
"""

from __future__ import annotations

import asyncio
import time
import uuid as _uuid
from dataclasses import dataclass, field
from typing import Any

from ans_nerves.captcha.solver import CapSolverClient, CaptchaToken
from ans_nerves.config import get_config
from ans_nerves.coordinator.coordinator import CrossEyeCoordinator, RoutedSignal
from ans_nerves.decomposer.decomposer import GoalDecomposer, GoalSpec, SubGoal
from ans_nerves.eyes.base import EyeReport
from ans_nerves.eyes.dom_reader import DomReaderEye
from ans_nerves.eyes.error_detector import ErrorDetectorEye
from ans_nerves.eyes.goal_verifier import GoalVerifierEye
from ans_nerves.eyes.page_diff import PageDiffEye
from ans_nerves.eyes.vision import VisionEye
from ans_nerves.eyes.som_annotator import annotate_screenshot
from ans_nerves.grpc_client import GrpcClient, get_grpc_client
from ans_nerves.logging import get_logger
from ans_nerves.planner.planner import AgentPlanner, PlannedAction
from ans_nerves.scoring.intelligence import ActionRecord, DecisionIntelligence

logger = get_logger(__name__)

_DEFAULT_MAX_STEPS = 50
_DEFAULT_MAX_ERRORS_PER_SUBGOAL = 3


@dataclass
class LoopState:
    """Mutable state tracked across the execution loop."""

    goal_spec: GoalSpec | None = None
    current_sub_goal_index: int = 0
    step_count: int = 0
    total_errors: int = 0
    action_history: list[str] = field(default_factory=list)
    last_page_url: str = ""
    last_page_title: str = ""
    last_distilled_dom: dict | None = None
    previous_distilled_dom: dict | None = None
    last_screenshot_b64: str | None = None
    last_diff: dict | None = None
    last_error: str = ""
    # ISSUE 1: Cache last vision report so we can reuse it when the page
    # hasn't changed (diff shows no_change or cosmetic_change).
    last_vision_report: Any = None  # EyeReport from previous Vision run
    last_vision_step: int = 0
    # ISSUE 2: Track last action success explicitly for deterministic
    # error detector triggering (separate from gRPC exception tracking).
    last_action_succeeded: bool = True
    # ISSUE 4: Smart early termination — track productive vs wasted steps
    # per sub-goal. Productive = diff shows meaningful change; wasted = no_change.
    productive_steps: int = 0
    wasted_steps: int = 0
    steps_since_verifier_confirm: int = 0
    # Hard action deduplication — tracks (action_type, selector) pairs that
    # already failed so the agent never retries them (code-level enforcement,
    # not just a soft LLM instruction).
    failed_action_keys: set = field(default_factory=set)
    done: bool = False
    final_summary: str = ""


@dataclass
class LoopResult:
    """Final result of an agent loop run."""

    success: bool = False
    total_steps: int = 0
    sub_goals_completed: int = 0
    total_sub_goals: int = 0
    total_errors: int = 0
    total_cost_cents: float = 0.0
    summary: str = ""
    records_stored: list[str] = field(default_factory=list)
    alerts: list[str] = field(default_factory=list)


class AgentLoop:
    """Main execution loop tying together all components.

    Usage:
        loop = AgentLoop(session_id="s1")

        result = await loop.run(
            goal="Search for flights from Delhi to Mumbai on June 5",
            context={"budget_cents": 500},
        )
    """

    def __init__(
        self,
        session_id: str,
        *,
        grpc: GrpcClient | None = None,
        decomposer: GoalDecomposer | None = None,
        coordinator: CrossEyeCoordinator | None = None,
        planner: AgentPlanner | None = None,
        intelligence: DecisionIntelligence | None = None,
        max_steps: int = _DEFAULT_MAX_STEPS,
        max_errors_per_subgoal: int = _DEFAULT_MAX_ERRORS_PER_SUBGOAL,
    ) -> None:
        self.session_id = session_id
        self._browser_session_id: str = ""  # daemon-issued UUID, set in run()
        self._grpc = grpc or get_grpc_client()
        self._decomposer = decomposer or GoalDecomposer()
        self._coordinator = coordinator or CrossEyeCoordinator()
        self._intelligence = intelligence or DecisionIntelligence(grpc_client=self._grpc)
        self._planner = planner or AgentPlanner(intelligence=self._intelligence)
        self._max_steps = max_steps
        self._max_errors_per_subgoal = max_errors_per_subgoal

        # Load runtime config for fast-mode toggles
        self._runtime = get_config().runtime
        logger.debug(
            "agent_loop: mode=%s fast_coord=%s skip_vision=%s skip_diff=%s",
            self._runtime.mode,
            self._runtime.fast_coordinator,
            self._runtime.skip_vision_in_fast,
            self._runtime.skip_diff_in_fast,
        )

        # CAPTCHA auto-solver (no-op if CAPSOLVER_API_KEY not set)
        self._captcha_solver = CapSolverClient()

        # Eyes (created once, reused)
        self._eyes = {
            "dom_reader": DomReaderEye(),
            "vision": VisionEye(),
            "page_diff": PageDiffEye(),
            "goal_verifier": GoalVerifierEye(),
            "error_detector": ErrorDetectorEye(),
        }

    # ── Public API ─────────────────────────────────────────────

    async def run(
        self,
        goal: str,
        context: dict | None = None,
        *,
        daemon_goal_id: str | None = None,
    ) -> LoopResult:
        """Execute the full agent loop for a goal.

        If daemon_goal_id is provided, uses the daemon's UUID instead
        of generating a hash-based ID.
        """
        state = LoopState()
        t_start = time.monotonic()

        logger.info("agent_loop: starting goal='%s' session=%s", goal[:80], self.session_id)

        effective_goal_id = daemon_goal_id or str(_uuid.uuid4())
        result = LoopResult(success=False, total_sub_goals=0)

        try:
            # 0. Create browser session
            session = await self._grpc.create_session(
                goal_id=effective_goal_id,
            )
            self._browser_session_id = session.session_id
            logger.info("agent_loop: browser session=%s", self._browser_session_id)

            # 1-6. Run full pipeline
            result = await self._run_impl(goal, context, state, t_start, effective_goal_id)

        except Exception as exc:
            logger.warning("agent_loop: failed: %s", exc)
            result = LoopResult(
                success=False,
                summary=f"Failed: {exc}",
                total_sub_goals=0,
            )

        finally:
            # Always report to daemon
            if daemon_goal_id:
                try:
                    status = "completed" if result.success else "failed"
                    await self._grpc.update_goal_progress(
                        goal_id=daemon_goal_id,
                        progress=1.0 if result.success else 0.0,
                        status=status,
                    )
                except Exception:
                    pass

        return result

    async def close(self) -> None:
        """Close the browser session. Call when done with the AgentLoop."""
        if self._browser_session_id:
            try:
                await self._grpc.close_session(self._browser_session_id, reason="loop done")
            except Exception:
                pass

    async def _run_impl(
        self,
        goal: str,
        context: dict | None,
        state: LoopState,
        t_start: float,
        goal_id: str,
    ) -> LoopResult:
        # 1. Decompose goal
        state.goal_spec = await self._decomposer.decompose(goal, context)
        if not state.goal_spec.sub_goals:
            logger.warning("agent_loop: no sub-goals produced")
            return LoopResult(
                success=False,
                summary="Goal decomposition produced no sub-goals.",
                total_sub_goals=0,
            )

        # Cap sub-goals to prevent over-decomposition
        state.goal_spec.sub_goals = state.goal_spec.sub_goals[:5]
        sg_count = len(state.goal_spec.sub_goals)

        # Push LLM-decomposed sub-goals back to daemon
        if goal_id:
            try:
                sub_goals_dict = [
                    {
                        "id": sg.id, "description": sg.description,
                        "success_criteria": sg.success_criteria,
                        "depends_on": sg.depends_on, "status": sg.status,
                    }
                    for sg in state.goal_spec.sub_goals
                ]
                await self._grpc.update_goal_progress(
                    goal_id=goal_id, progress=0.0, status="active",
                    sub_goals=sub_goals_dict,
                )
                logger.info("agent_loop: pushed %d LLM sub-goals to daemon", sg_count)
            except Exception:
                pass

        # 2. Iterate sub-goals
        for i, sub_goal in enumerate(state.goal_spec.sub_goals):
            if state.done:
                break

            state.current_sub_goal_index = i
            logger.info(
                "agent_loop: sub_goal %d/%d: '%s'",
                i + 1, sg_count, sub_goal.description[:80],
            )

            sub_result = await self._execute_sub_goal(
                goal=goal,
                goal_id=goal_id,
                sub_goal=sub_goal,
                state=state,
                context=context or {},
            )

            # Handle re-decomposition: stuck sub-goal was broken into finer steps.
            replacement: list = sub_result.get("replace_with") or []
            if replacement:
                logger.info(
                    "agent_loop: inserting %d finer sub-goals for '%s'",
                    len(replacement), sub_goal.description[:60],
                )
                # Insert finer sub-goals after the current position
                for offset, finer_sg in enumerate(replacement):
                    state.goal_spec.sub_goals.insert(i + 1 + offset, finer_sg)
                sg_count = len(state.goal_spec.sub_goals)
                continue

            # Report incremental progress to daemon
            if goal_id:
                try:
                    progress = (i + 1) / sg_count if sg_count > 0 else 0.0
                    await self._grpc.update_goal_progress(
                        goal_id=goal_id,
                        progress=min(progress, 0.99),
                        status="active",
                    )
                except Exception:
                    pass

            if sub_result.get("escalate"):
                return LoopResult(
                    success=False,
                    total_steps=state.step_count,
                    sub_goals_completed=i,
                    total_sub_goals=sg_count,
                    total_errors=state.total_errors,
                    summary=f"Escalated at sub-goal {i+1}/{sg_count}: {sub_result.get('reason', '')}",
                    alerts=state.goal_spec.risk_factors or [],
                )

        # 3. Build result
        elapsed = time.monotonic() - t_start
        completed = state.current_sub_goal_index + (1 if not state.done else 0)

        logger.info(
            "agent_loop: done steps=%d completed=%d/%d errors=%d elapsed=%.1fs",
            state.step_count, min(completed, sg_count), sg_count,
            state.total_errors, elapsed,
        )

        return LoopResult(
            success=completed >= sg_count,
            total_steps=state.step_count,
            sub_goals_completed=min(completed, sg_count),
            total_sub_goals=sg_count,
            total_errors=state.total_errors,
            summary=state.final_summary or f"Completed {min(completed, sg_count)}/{sg_count} sub-goals in {state.step_count} steps.",
        )

    # ── Sub-goal execution ────────────────────────────────────

    async def _execute_sub_goal(
        self,
        goal: str,
        goal_id: str,
        sub_goal: SubGoal,
        state: LoopState,
        context: dict,
    ) -> dict:
        """Execute a single sub-goal: perceive → decide → act → learn → verify."""
        errors_this_subgoal = 0
        executed_captcha_this_subgoal = 0
        sub_goal_done = False

        # Reset per-sub-goal tracking for smart early termination (ISSUE 4).
        state.productive_steps = 0
        state.wasted_steps = 0
        state.steps_since_verifier_confirm = 0

        while not sub_goal_done and state.step_count < self._max_steps:
            state.step_count += 1

            # Track steps since last verifier confirmation (ISSUE 4).
            state.steps_since_verifier_confirm = getattr(state, "steps_since_verifier_confirm", 0) + 1

            # ── Perceive ──────────────────────────────────────
            # Snapshot URL before gathering page data so we can detect
            # when the agent navigated to a new page (ISSUE 3).
            _url_before_step = state.last_page_url
            page_data = await self._gather_page_data(state)
            page_data["goal_context"] = goal  # populated in caller scope

            # Track consecutive "no_change" diffs for stagnation detection.
            diff = page_data.get("diff", {}) or {}
            diff_pct = diff.get("visual_diff_percentage", 1.0)
            diff_summary = diff.get("summary", "")
            if diff_summary == "no_change" or diff_pct == 0.0:
                state.consecutive_no_change = getattr(state, "consecutive_no_change", 0) + 1
                # Track wasted steps for smart early termination (ISSUE 4).
                state.wasted_steps = getattr(state, "wasted_steps", 0) + 1
            else:
                state.consecutive_no_change = 0
                # Track productive steps — diff shows a meaningful page change.
                # Cosmetic changes are neutral (neither productive nor wasted).
                if diff_summary not in ("", "cosmetic_change"):
                    state.productive_steps = getattr(state, "productive_steps", 0) + 1

            # Annotate screenshot with Set-of-Marks numbered bounding boxes
            # so the Vision Eye and Planner can reference elements by visual index.
            som_elements = self._extract_interactive_elements(page_data)
            if page_data.get("screenshot_base64"):
                page_data["som_screenshot_base64"] = annotate_screenshot(
                    page_data["screenshot_base64"], som_elements,
                )

            # DOM Reader runs first (deterministic, no LLM)
            eye_reports: list[EyeReport] = []
            dom_report = await self._eyes["dom_reader"].observe(
                self.session_id, page_data,
            )
            eye_reports.append(dom_report)

            # Vision + Error Detector run in parallel (LLM-bound)
            # PageDiff LLM eliminated: structural diff feeds coordinator fallback.
            # Vision gating: fast+skip → never; thorough+clean → every 3rd cycle
            dom_content = dom_report.content if isinstance(dom_report.content, dict) else {}
            has_overlays = bool(dom_content.get("distraction_flags"))
            is_fast = self._runtime.mode == "fast"

            parallel_tasks = []
            _run_vision = page_data.get("screenshot_base64") is not None
            if _run_vision and is_fast and self._runtime.skip_vision_in_fast:
                _run_vision = False
            elif _run_vision and is_fast and not has_overlays:
                # Fast mode: run vision on steps 1-2 (orientation) then
                # throttle to every 3rd step.  Thorough mode always runs
                # vision — no throttling.
                _run_vision = state.step_count <= 2 or state.step_count % 3 == 0

            # ISSUE 1: Skip vision when the page hasn't changed meaningfully.
            # If the diff shows no_change or cosmetic_change and we have a
            # recent cached vision report (within last 2 steps), reuse it
            # instead of making a redundant GPT-4o-mini call.
            #
            # CRITICAL OVERRIDE: never skip Vision when DOM Reader detected
            # overlays/popups/modals (distraction_flags). CSS-only overlays
            # (display:none→block, opacity changes) often appear without DOM
            # structure changes — Page Diff classifies as "cosmetic_change"
            # but Vision is the ONLY eye that can visually confirm them.
            # Skipping Vision here would blind ANS to the exact thing it's
            # supposed to catch (immune system / distraction blocking).
            if _run_vision:
                _is_stale_page = diff_summary in ("no_change", "cosmetic_change")
                _has_recent_vision = (
                    getattr(state, "last_vision_report", None) is not None
                    and (state.step_count - getattr(state, "last_vision_step", 0)) <= 2
                )
                if _is_stale_page and _has_recent_vision and not has_overlays:
                    _run_vision = False
                    logger.debug(
                        "agent_loop: step %d reusing cached vision (page unchanged, last vision step %d)",
                        state.step_count, state.last_vision_step,
                    )
                    eye_reports.append(state.last_vision_report)

            if _run_vision:
                parallel_tasks.append(self._eyes["vision"].observe(self.session_id, page_data))

            # PageDiff eye: semantic interpretation of structural diff data.
            # Runs when the diff shows meaningful page changes.
            _diff_data = page_data.get("diff", {}) or {}
            if _diff_data and _diff_data.get("summary", "no_change") != "no_change":
                parallel_tasks.append(
                    self._eyes["page_diff"].observe(self.session_id, page_data)
                )

            # ── Error Detector Gating (ISSUE 2) ──────────────────
            # Deterministic triggers — zero LLM cost to evaluate:
            #   1. gRPC exception on the previous action (state.last_error)
            #   2. gRPC returned success=false (structured failure with error_message)
            #   3. Silent failure: 2+ consecutive no_change diffs (actions have no effect)
            # Only fire the LLM-backed error detector when something is actually
            # wrong; skip it entirely when all three conditions are clear.
            _silent_fail = getattr(state, "consecutive_no_change", 0) >= 2
            _action_failed = not getattr(state, "last_action_succeeded", True)
            if state.last_error or _action_failed or _silent_fail:
                err_ctx: dict[str, Any] = {
                    "action_description": (
                        state.action_history[-1] if state.action_history else "unknown"
                    ),
                    "error_message": state.last_error or (
                        f"Silent failure: page unchanged for "
                        f"{getattr(state, 'consecutive_no_change', 0)} consecutive steps"
                    ),
                    "page_url": state.last_page_url,
                    "page_title": state.last_page_title,
                    "visible_text": page_data.get("visible_text", []),
                    "goal_context": goal,
                }
                parallel_tasks.append(
                    self._eyes["error_detector"].observe(self.session_id, err_ctx)
                )
            if parallel_tasks:
                parallel_results = await asyncio.gather(*parallel_tasks, return_exceptions=True)
                for r in parallel_results:
                    if isinstance(r, Exception):
                        logger.warning("Eye failed in parallel: %s", r)
                    else:
                        eye_reports.append(r)
                        # Cache vision reports for reuse when the page is unchanged (ISSUE 1).
                        # Do NOT cache when overlays are present — the page is volatile
                        # and a cached "no overlay" report would be dangerously stale.
                        if getattr(r, "eye_name", "") == "vision" and not has_overlays:
                            state.last_vision_report = r
                            state.last_vision_step = state.step_count

            # ── Synthesize ────────────────────────────────────
            # Use LLM synthesis when there are contradictions between eyes,
            # error conditions, or PageDiff has semantic insight. Otherwise
            # the deterministic fallback is sufficient.
            diff_data = state.last_diff or {}
            _has_contradictions = any(
                r.content.get("failure_type") or r.content.get("anomaly")
                for r in eye_reports
                if isinstance(r.content, dict)
            )
            _has_meaningful_diff = bool(
                diff_data.get("summary", "no_change") not in ("no_change", "cosmetic_change")
            )
            if (_has_contradictions or _has_meaningful_diff) and len(eye_reports) >= 2:
                signal = await self._coordinator.synthesize(eye_reports, goal_context=goal)
            else:
                signal = self._coordinator._fallback_synthesize(
                    eye_reports, diff_data=diff_data,
                )

            # ── Decide ────────────────────────────────────────
            dom_elements = self._extract_interactive_elements(page_data)
            page_type = self._extract_page_type(signal)

            # Extract Vision-confirmed target: if the Vision Eye identified
            # a specific element_index from the SOM-annotated screenshot with
            # high confidence, pass it as a directive to the planner.
            from ans_nerves.planner.planner import VisionConfirmedTarget
            vision_target = _extract_vision_target(eye_reports, dom_elements)

            planned = await self._planner.plan_next_action(
                goal_context=goal,
                sub_goal=sub_goal.description,
                sub_goal_criteria=sub_goal.success_criteria,
                unified_perception=signal.unified_perception,
                available_elements=dom_elements,
                action_history=state.action_history,
                last_error=state.last_error,
                page_type=page_type,
                vision_target=vision_target,
            )

            logger.info(
                "agent_loop: step %d action=%s selector=%s source=%s conf=%.2f",
                state.step_count, planned.action_type, planned.selector,
                planned.source, planned.confidence,
            )

            if planned.action_type == "ask_user":
                # ── Missing Information Gate ────────────────────
                # Planner detected form fields that need info not
                # provided in the goal. Pause and ask the user.
                missing_fields = planned.value or "additional details"
                await self._pause_for_human(
                    goal_id=goal_id,
                    block_reason=f"Need more info: {missing_fields}",
                    page_url=state.last_page_url,
                    sub_goal_description=sub_goal.description,
                    step_count=state.step_count,
                )
                # After human provides info via dashboard, retry
                state.last_error = None
                errors_this_subgoal = 0
                continue

            if planned.action_type == "escalate":
                # ── CAPTCHA Auto-Solve ──────────────────────────
                if page_type == "captcha" and executed_captcha_this_subgoal < 1:
                    captcha_token = await self._try_auto_solve_captcha(
                        page_data, page_data.get("url", "")
                    )
                    if captcha_token is not None:
                        executed_captcha_this_subgoal += 1
                        logger.info(
                            "agent_loop: CAPTCHA auto-solved, injecting token and retrying"
                        )
                        # Inject the token via evaluate, then retry the sub-goal
                        script = self._captcha_solver.build_injection_script(captcha_token)
                        try:
                            inject_result = await self._grpc.evaluate(
                                self._browser_session_id, script
                            )
                            logger.info(
                                "agent_loop: token injection result=%s",
                                str(inject_result)[:80],
                            )
                        except Exception as exc:
                            logger.warning(
                                "agent_loop: token injection failed: %s", exc
                            )
                        # Small wait for the callback to fire
                        await asyncio.sleep(1.5)
                        state.last_error = None
                        errors_this_subgoal = 0
                        continue  # Retry the sub-goal

                # ── Human-in-the-Loop Pause Gate ─────────────────
                # If the page is blocked by a human-solvable challenge
                # (CAPTCHA after auto-solve fails, login wall, paywall),
                # pause and wait for human intervention via the dashboard.
                _human_solvable_types = {"captcha", "login_wall", "paywall", "bot_block"}
                if page_type in _human_solvable_types:
                    await self._pause_for_human(
                        goal_id=goal_id,
                        block_reason=page_type,
                        page_url=state.last_page_url,
                        sub_goal_description=sub_goal.description,
                        step_count=state.step_count,
                    )
                    # Human intervened — reset error state and retry sub-goal
                    state.last_error = None
                    errors_this_subgoal = 0
                    continue

                return {"escalate": True, "reason": planned.reasoning}

            # ── Hard Deduplication Gate ────────────────────────
            # If this exact (action_type, selector) already failed, block it
            # at the code level and re-plan. The [FAILED] tag in action_history
            # is only a soft LLM hint — this is the hard enforcement.
            _replan_attempts = 0
            while (
                planned.action_type not in ("done", "wait", "navigate", "escalate", "ask_user")
                and (planned.action_type, planned.selector) in state.failed_action_keys
                and _replan_attempts < 2
            ):
                _replan_attempts += 1
                logger.warning(
                    "agent_loop: hard-blocked repeat of failed action %s '%s', "
                    "re-planning (attempt %d)",
                    planned.action_type, planned.selector, _replan_attempts,
                )
                state.last_error = (
                    f"BLOCKED: '{planned.action_type}' on '{planned.selector}' "
                    f"already failed. You MUST choose a DIFFERENT element. "
                    f"Failed selectors: {[s for _, s in state.failed_action_keys]}"
                )
                planned = await self._planner.plan_next_action(
                    goal_context=goal,
                    sub_goal=sub_goal.description,
                    sub_goal_criteria=sub_goal.success_criteria,
                    unified_perception=signal.unified_perception,
                    available_elements=dom_elements,
                    action_history=state.action_history,
                    last_error=state.last_error,
                    page_type=page_type,
                    vision_target=None,  # Disable vision override on re-plan
                )
                logger.info(
                    "agent_loop: re-planned → action=%s selector=%s conf=%.2f",
                    planned.action_type, planned.selector, planned.confidence,
                )

            # ── Execute (skip for terminal "done") ───────────
            if planned.action_type != "done":
                t_action = time.monotonic()
                try:
                    action_result = await self._execute_action(planned, dom_elements)
                    execution_time_ms = int((time.monotonic() - t_action) * 1000)
                    action_succeeded = action_result.get("success", False)
                    result_text = action_result.get("message", "")
                    exec_error = None

                    # Post-click validation: if the Rust stability wait detected
                    # that the page URL didn't change after a click, the click
                    # likely had no effect (wrong element, dead link, or overlay).
                    # Override success to trigger error detection and correction.
                    if action_succeeded and planned.action_type == "click":
                        err_msg = action_result.get("error_message", "")
                        if "did not cause" in err_msg or "unchanged" in err_msg:
                            action_succeeded = False
                            exec_error = err_msg
                            state.last_error = err_msg
                            errors_this_subgoal += 1
                            state.total_errors += 1
                            logger.warning(
                                "agent_loop: click had no visible effect step=%d selector=%s",
                                state.step_count, planned.selector,
                            )

                    if not action_succeeded:
                        err_msg = action_result.get("error_message", "")
                        if err_msg:
                            exec_error = err_msg
                            state.last_error = err_msg
                            errors_this_subgoal += 1
                            state.total_errors += 1
                            logger.warning(
                                "agent_loop: action returned success=false step=%d error=%s",
                                state.step_count, err_msg[:120],
                            )
                except Exception as exc:
                    execution_time_ms = int((time.monotonic() - t_action) * 1000)
                    action_succeeded = False
                    result_text = ""
                    exec_error = str(exc)
                    state.last_error = exec_error
                    errors_this_subgoal += 1
                    state.total_errors += 1

                    logger.warning(
                        "agent_loop: action failed step=%d error=%s",
                        state.step_count, exec_error[:120],
                    )
            else:
                execution_time_ms = 0
                action_succeeded = True
                result_text = ""
                exec_error = None

            # Track explicit action success for deterministic error detection (ISSUE 2).
            # Separates structured gRPC failures (success=false) from transport exceptions.
            state.last_action_succeeded = action_succeeded

            # ── Self-correction: validate expected outcome ──────
            # After every click, check whether we navigated to a third-party
            # auth domain. This fires ALWAYS — not just when expected_outcome
            # is provided — to prevent memory poisoning from "successful"
            # clicks that actually redirected to Google/Facebook OAuth.
            _THIRD_PARTY_AUTH_DOMAINS = [
                "accounts.google.com", "login.microsoftonline.com",
                "www.facebook.com", "github.com/login",
                "appleid.apple.com",
            ]
            if action_succeeded and planned.action_type == "click" and state.last_page_url:
                new_state = action_result.get("new_state") or {}
                post_url = (new_state.get("url", "") if isinstance(new_state, dict)
                           else "")
                if post_url and post_url != state.last_page_url:
                    expected_lower = (planned.expected_outcome or "").lower()
                    post_domain = post_url.split("/")[2] if "/" in post_url else ""
                    current_domain = state.last_page_url.split("/")[2] if "/" in state.last_page_url else ""
                    # Third-party auth redirect detection (covers Google,
                    # Facebook, GitHub, Apple, Microsoft)
                    _is_auth_redirect = any(
                        tp in post_domain for tp in _THIRD_PARTY_AUTH_DOMAINS
                    )
                    _user_wanted_auth = any(
                        p in expected_lower
                        for p in ["google", "facebook", "github", "apple", "microsoft"]
                    )
                    if _is_auth_redirect and not _user_wanted_auth:
                        action_succeeded = False
                        exec_error = (
                            f"Unexpected navigation to third-party auth "
                            f"({post_domain}). The user wants direct form "
                            f"filling, not social login."
                        )
                        state.last_error = exec_error
                        errors_this_subgoal += 1
                        state.total_errors += 1
                        logger.warning(
                            "agent_loop: third-party auth redirect step=%d to=%s",
                            state.step_count, post_domain,
                        )
                        # Navigate back to the original page so the agent
                        # can retry with the correct form elements.
                        try:
                            await self._grpc.navigate(
                                self._browser_session_id, state.last_page_url,
                            )
                            logger.info(
                                "agent_loop: navigated back to %s",
                                state.last_page_url[:80],
                            )
                        except Exception:
                            pass
                    # Any other unexpected domain change
                    elif ("navigat" not in expected_lower
                          and "go to" not in expected_lower
                          and post_domain and current_domain
                          and post_domain != current_domain):
                        action_succeeded = False
                        exec_error = (
                            f"Navigated to {post_domain} but expected to stay on "
                            f"{current_domain}. Expected: "
                            f"{(planned.expected_outcome or 'stay on page')[:100]}"
                        )
                        state.last_error = exec_error
                        errors_this_subgoal += 1
                        state.total_errors += 1
                        logger.warning(
                            "agent_loop: domain mismatch step=%d from=%s to=%s",
                            state.step_count, current_domain, post_domain,
                        )

            # Mark failed actions so the planner knows NOT to repeat them
            action_entry = f"{planned.action_type} {planned.selector} {planned.value}".strip()
            if not action_succeeded:
                action_entry = f"[FAILED] {action_entry}"
                # Hard dedup: record the (action_type, selector) pair so
                # the code-level guard blocks re-execution (Component 4).
                if planned.selector:
                    state.failed_action_keys.add(
                        (planned.action_type, planned.selector)
                    )
            state.action_history.append(action_entry)
            # Keep only last 20 entries so context stays manageable
            if len(state.action_history) > 20:
                state.action_history = state.action_history[-20:]

            # ── Learn ─────────────────────────────────────────
            record = ActionRecord(
                session_id=self.session_id,
                goal_id=goal_id,
                action_type=planned.action_type,
                selector=planned.selector,
                value=planned.value,
                tool=planned.tool,
                context_type=page_type,
                goal_description=goal,
                page_type=page_type,
                action_succeeded=action_succeeded,
                results_produced=result_text,
                error_message=exec_error,
                goal_advanced=action_succeeded and not exec_error,
                criterion_met=None,
                sub_goal_completed=False,
                execution_time_ms=execution_time_ms,
            )
            await self._intelligence.record_action(record)

            # ── Live View Update ──────────────────────────────
            # Push current screenshot + action info to dashboard
            # via goal progress update. The gateway bridge detects
            # agent_step messages and pushes WS events to clients.
            await self._push_live_step(
                goal_id=goal_id,
                sub_goal=sub_goal.description,
                action=planned.action_type,
                selector=planned.selector or "",
                step=state.step_count,
                page_url=state.last_page_url,
                page_title=state.last_page_title,
                page_type=page_type,
            )

            # ── Verify sub-goal (ISSUE 3: adaptive triggers) ───
            # Verify immediately after significant page changes instead of
            # waiting for a fixed interval. This catches errors on step 1
            # instead of letting them accumulate until step 5.
            consecutive_no_change = getattr(state, "consecutive_no_change", 0)
            _diff_summary = (state.last_diff or {}).get("summary", "")
            _sig_change = _diff_summary in (
                "navigation", "form_update", "error_state", "content_update",
            )
            _low_confidence = planned.confidence < 0.4
            _url_changed = bool(_url_before_step and _url_before_step != state.last_page_url)
            _should_verify = (
                errors_this_subgoal >= 2
                or planned.action_type == "done"
                or _sig_change
                or _low_confidence
                or _url_changed
                or consecutive_no_change >= 3
                # Backstop: periodic verify every 5 steps for pages without
                # diff data, or when no other trigger has fired.
                or state.step_count % 5 == 0
            )
            if _should_verify:
                verifier_report = await self._eyes["goal_verifier"].observe(
                    self.session_id, {
                        "sub_goal_description": sub_goal.description,
                        "success_criteria": sub_goal.success_criteria,
                        "page_url": state.last_page_url,
                        "page_title": state.last_page_title,
                        "visible_text": page_data.get("visible_text", []),
                        "dom_summary": json_safe(page_data.get("distilled_dom", {})),
                        "diff_summary": state.last_diff.get("summary", "") if state.last_diff else "",
                        "screenshot_base64": state.last_screenshot_b64 or "",
                    },
                )

                if verifier_report.content.get("criteria_met") and verifier_report.confidence > 0.6:
                    sub_goal_done = True
                    state.steps_since_verifier_confirm = 0  # Reset counter on confirmation (ISSUE 4)
                    state.final_summary = (
                        f"Sub-goal '{sub_goal.description[:60]}' verified: "
                        f"{verifier_report.content.get('reasoning', '')[:200]}"
                    )
                    # Backfill the last action record with verified short-term
                    # outcome so warm-start memory learns which actions actually
                    # completed sub-goals.
                    try:
                        await self._intelligence.update_record_outcome(
                            session_id=self.session_id,
                            goal_id=goal_id,
                            goal_advanced=True,
                            criterion_met=True,
                            sub_goal_completed=True,
                        )
                    except Exception:
                        logger.debug("agent_loop: update_record_outcome failed", exc_info=True)

            # Reset error state on success
            if action_succeeded:
                state.last_error = ""

            # ── Smart Early Termination (ISSUE 4) ─────────────────
            # Cut sub-goal short in common failure modes instead of
            # burning through the full 50-step budget per sub-goal.
            _wasted = getattr(state, "wasted_steps", 0)
            _productive = getattr(state, "productive_steps", 0)
            _no_verifier = getattr(state, "steps_since_verifier_confirm", 0)

            # 1. Zero progress after 15 wasted steps — agent is stuck,
            #    escalate immediately instead of waiting to step 50.
            if _wasted >= 15 and _productive == 0:
                return {
                    "escalate": True,
                    "reason": (
                        f"Sub-goal '{sub_goal.description[:60]}' stuck: "
                        f"{_wasted} steps with zero diff changes, "
                        f"no productive actions detected"
                    ),
                }

            # 2. Thirty steps without verifier confirmation — the agent
            #    is spinning without making verifiable progress.
            #    Re-decompose into finer sub-goals.
            if _no_verifier >= 30:
                finer = await self._decomposer.decompose_single_sub_goal(
                    sub_goal_description=sub_goal.description,
                    full_goal=goal,
                    context=context,
                )
                if finer and len(finer) >= 2:
                    logger.info(
                        "agent_loop: re-decomposed after %d unverified steps",
                        _no_verifier,
                    )
                    return {
                        "escalate": False,
                        "done": False,
                        "replace_with": finer,
                        "reason": (
                            f"Re-decomposed '{sub_goal.description[:60]}' after "
                            f"{_no_verifier} steps without verifier confirmation"
                        ),
                    }
                # Fall through to error gating below if re-decomposition fails.

            # 3. Wasted steps dominate productive steps — agent is stuck in
            #    a loop of ineffective actions.
            if _wasted > _productive * 2 and _productive > 0:
                return {
                    "escalate": True,
                    "reason": (
                        f"Sub-goal '{sub_goal.description[:60]}' stuck: "
                        f"{_wasted} wasted steps vs {_productive} productive "
                        f"(ratio {_wasted / _productive:.1f}:1)"
                    ),
                }

            # ── Error Gating — try re-decomposition before escalating ──
            stuck_errors = errors_this_subgoal >= self._max_errors_per_subgoal
            stuck_stagnation = getattr(state, "consecutive_no_change", 0) >= 5
            if stuck_errors or stuck_stagnation:
                reason = (
                    f"errors={errors_this_subgoal}" if stuck_errors
                    else f"stagnation={getattr(state, 'consecutive_no_change', 0)}"
                )
                logger.warning(
                    "agent_loop: sub-goal stuck (%s), attempting re-decomposition",
                    reason,
                )
                finer = await self._decomposer.decompose_single_sub_goal(
                    sub_goal_description=sub_goal.description,
                    full_goal=goal,
                    context=context,
                )
                if finer and len(finer) >= 2:
                    logger.info(
                        "agent_loop: re-decomposed stuck sub-goal into %d finer steps",
                        len(finer),
                    )
                    return {
                        "escalate": False,
                        "done": False,
                        "replace_with": finer,
                        "reason": (
                            f"Re-decomposed '{sub_goal.description[:60]}' into "
                            f"{len(finer)} granular steps"
                        ),
                    }
                return {
                    "escalate": True,
                    "reason": (
                        f"Sub-goal '{sub_goal.description[:80]}' stuck "
                        f"({reason}, max retries exhausted)"
                    ),
                }

        return {"escalate": False, "done": sub_goal_done}

    # ── Helpers ────────────────────────────────────────────────

    async def _gather_page_data(self, state: LoopState) -> dict[str, Any]:
        """Gather current page state from gRPC daemon."""
        page_data: dict[str, Any] = {}

        try:
            dom = await self._grpc.get_distilled_dom(self._browser_session_id, mode="all_fields")
            state.last_distilled_dom = dom
            page_data["distilled_dom"] = dom
            if dom:
                state.last_page_url = dom.get("url", state.last_page_url)
                state.last_page_title = dom.get("title", state.last_page_title)
                # Extract visible_text from distilled DOM elements so the
                # verifier and error detector receive real page content.
                page_data["visible_text"] = [
                    el.get("text", "")
                    for el in (dom.get("elements", []) or [])
                    if el.get("text", "").strip()
                ][:100]
        except Exception:
            pass

        try:
            screenshot = await self._grpc.capture_screenshot(self._browser_session_id)
            state.last_screenshot_b64 = screenshot.get("data", "")
            page_data["screenshot_base64"] = state.last_screenshot_b64
        except Exception:
            pass

        try:
            if state.previous_distilled_dom and state.last_distilled_dom:
                diff = await self._grpc.compute_diff(
                    self._browser_session_id,
                    before=state.previous_distilled_dom,
                    after=state.last_distilled_dom,
                )
                state.last_diff = diff
                page_data["diff"] = diff
        except Exception:
            pass
        state.previous_distilled_dom = state.last_distilled_dom

        page_data["page_url"] = state.last_page_url
        page_data["page_title"] = state.last_page_title

        return page_data

    async def _execute_action(self, planned: PlannedAction, dom_elements: list[dict] | None = None) -> dict:
        """Execute a planned action via gRPC with parameter validation."""
        sid = self._browser_session_id
        action_type = planned.action_type

        # Validate navigate has URL
        if action_type == "navigate":
            if not planned.value:
                raise ValueError("Navigate action requires a non-empty URL")
            return await self._grpc.navigate(sid, planned.value)

        # Evaluate: execute JS in page context, script comes from value field
        if action_type == "evaluate":
            return await self._grpc.execute_action(
                sid,
                action_type="evaluate",
                value=planned.value,
            )

        # Validate selector exists in DOM for click/fill/type/select
        if action_type in ("click", "fill", "type", "select", "submit"):
            if not planned.selector:
                raise ValueError(f"Action '{action_type}' requires a selector")

            # Fallback: if selector not in DOM, try element_index first.
            if dom_elements and planned.selector not in {e.get("selector") for e in dom_elements}:
                logger.warning("agent_loop: selector '%s' not in DOM, trying fallback", planned.selector)
                resolved = False
                # Prefer resolution by element_index (Set-of-Marks)
                if getattr(planned, "element_index", None) is not None:
                    for e in dom_elements:
                        if e.get("element_index") == planned.element_index:
                            planned.selector = e.get("selector", "")
                            logger.info(
                                "agent_loop: resolved element_index=%d → selector='%s'",
                                planned.element_index, planned.selector,
                            )
                            resolved = True
                            break
                if not resolved:
                    # Try fuzzy match: check if any DOM element's selector
                    # contains the planned selector as a substring.
                    for e in dom_elements:
                        dom_sel = e.get("selector", "")
                        if (planned.selector in dom_sel
                                or dom_sel in planned.selector):
                            planned.selector = dom_sel
                            planned.confidence *= 0.7
                            resolved = True
                            logger.info(
                                "agent_loop: fuzzy-matched selector → '%s'",
                                dom_sel,
                            )
                            break
                    if not resolved:
                        raise ValueError(
                            f"Selector '{planned.selector}' not found in DOM "
                            f"(no fuzzy match). Available: "
                            f"{[e.get('selector', '')[:40] for e in dom_elements[:10]]}"
                        )

        # Map 'fill' → 'type' — the Rust daemon supports 'type' for text input
        daemon_action = "type" if action_type == "fill" else action_type

        return await self._grpc.execute_action(
            sid,
            action_type=daemon_action,
            selector=planned.selector,
            value=planned.value,
        )

    async def _try_auto_solve_captcha(
        self, page_data: dict, page_url: str
    ) -> CaptchaToken | None:
        """Attempt to detect and solve a CAPTCHA on the current page.

        Returns a CaptchaToken if solved, None otherwise.
        Safe to call even if CapSolver is not configured — returns None.
        """
        if not self._captcha_solver.configured:
            return None

        # Get DOM HTML for sitekey extraction
        dom_html = ""
        try:
            dom = page_data.get("distilled_dom", {})
            if isinstance(dom, dict):
                dom_html = str(dom)
        except Exception:
            pass

        return await self._captcha_solver.solve_captcha(
            page_url=page_url,
            dom_html=dom_html,
        )

    async def _push_live_step(
        self,
        goal_id: str,
        sub_goal: str,
        action: str,
        selector: str,
        step: int,
        page_url: str,
        page_title: str,
        page_type: str,
    ) -> None:
        """Push a lightweight live-view update to the dashboard via WebSocket."""
        import json as _json
        try:
            msg = _json.dumps({
                "type": "agent_step",
                "session_id": self._browser_session_id,
                "action": f"{action} {selector}".strip(),
                "step": step,
                "page_url": page_url[:120],
                "page_title": page_title[:80],
                "page_type": page_type,
                "sub_goal": sub_goal[:100],
            })
            await self._grpc.update_goal_progress(
                goal_id, progress=0.0, status="active", message=msg,
            )
        except Exception:
            pass  # Live view is best-effort; never block the agent loop

    async def _pause_for_human(
        self,
        goal_id: str,
        block_reason: str,
        page_url: str,
        sub_goal_description: str,
        step_count: int,
    ) -> None:
        """Pause execution and wait for human intervention via the dashboard.

        Sets the goal status to Blocked with recovery context packed into
        the message field. The gateway detects this and pushes a
        ``session_paused`` event to WebSocket clients.

        Then polls ``get_goal_state`` every 2 seconds until the goal is
        unblocked (human clicks "Resume" in the dashboard).
        """
        import json as _json

        pause_msg = _json.dumps({
            "session_id": self._browser_session_id,
            "block_reason": block_reason,
            "page_url": page_url,
            "sub_goal": sub_goal_description[:200],
            "step_count": step_count,
        })

        logger.info(
            "agent_loop: PAUSED for human — reason=%s url=%s",
            block_reason, page_url[:80],
        )

        try:
            await self._grpc.update_goal_progress(
                goal_id,
                progress=0.0,
                status="blocked",
                message=pause_msg,
            )
        except Exception as exc:
            logger.warning("agent_loop: failed to set goal blocked: %s", exc)
            # Fall through — still poll in case the gateway detected it

        # Poll until human resumes
        poll_interval_s = 2.0
        max_wait_s = 600.0  # 10-minute timeout
        waited = 0.0
        while waited < max_wait_s:
            await asyncio.sleep(poll_interval_s)
            waited += poll_interval_s
            try:
                state = await self._grpc.get_goal_state(goal_id)
                status = (
                    state.get("status", "")
                    if isinstance(state, dict)
                    else ""
                )
                if status != "blocked":
                    logger.info(
                        "agent_loop: RESUMED by human after %.0fs — status=%s",
                        waited, status,
                    )
                    return
            except Exception:
                pass

        logger.warning(
            "agent_loop: pause timed out after %.0fs, resuming anyway",
            waited,
        )

    @staticmethod
    def _extract_interactive_elements(page_data: dict) -> list[dict]:
        """Extract interactive elements from DOM data for the planner.

        Includes label, placeholder, name, input_type, and autocomplete
        attributes so the planner can identify form fields unambiguously.
        Caps at 80 elements sorted by goal relevance.
        """
        dom = page_data.get("distilled_dom", {})
        if isinstance(dom, str):
            return []
        seen = set()
        result = []
        for source in (dom.get("interactive", []), dom.get("elements", [])):
            for e in (source or []):
                sel = e.get("selector", "")
                if not sel or sel in seen:
                    continue
                seen.add(sel)
                if e.get("is_visible", True) and e.get("is_enabled", True):
                    text = e.get("text", "") or e.get("label", "") or ""
                    # Allow more text for form labels — 120 chars min
                    if len(text) > 120:
                        text = text[:117] + "..."
                    # Form attributes live in `attributes` map for DistilledElement,
                    # and as first-class fields for InteractiveElement (proto field 10).
                    attrs = e.get("attributes", {}) or {}
                    entry = {
                        "selector": sel,
                        "element_index": e.get("element_index"),
                        "tag": e.get("tag", ""),
                        "text": text,
                        "label": e.get("label", ""),
                        "placeholder": (
                            e.get("placeholder", "")
                            or attrs.get("placeholder", "")
                        ),
                        "name": e.get("name", "") or attrs.get("name", ""),
                        "input_type": attrs.get("type", e.get("input_type", "")),
                        "autocomplete": attrs.get("autocomplete", e.get("autocomplete", "")),
                        "current_value": (
                            e.get("current_value", "")
                            or attrs.get("value", "")
                        ),
                        "aria_label": attrs.get("aria-label", e.get("aria_label", "")),
                        "aria_role": attrs.get("role", e.get("aria_role", "")),
                        "is_visible": True,
                        "is_enabled": True,
                    }
                    # Tag social login buttons so the LLM clearly
                    # recognises them and avoids them (Component 1c).
                    if _is_social_login_element(entry):
                        entry["text"] = f"[SOCIAL_LOGIN] {entry['text']}"
                    result.append(entry)
        # Filter to top 80 by relevance, then sort by DOM order
        # (element_index) so the planner sees fields in visual
        # top-to-bottom order — matching the "fill top-to-bottom"
        # instruction in the system prompt.
        _dom = dom
        relevance_map: dict[str, float] = {}
        for source in (_dom.get("interactive", []), _dom.get("elements", [])):
            for e in (source or []):
                sel = e.get("selector", "")
                if sel and "goal_relevance_score" in e:
                    try:
                        relevance_map[sel] = float(e["goal_relevance_score"])
                    except (TypeError, ValueError):
                        pass
        if len(result) > 80:
            result.sort(
                key=lambda e: relevance_map.get(e["selector"], 0),
                reverse=True,
            )
            result = result[:80]
        # Final sort: DOM order (element_index) for visual consistency.
        result.sort(key=lambda e: e.get("element_index") or 0)
        return result

    @staticmethod
    def _extract_page_type(signal: RoutedSignal) -> str:
        """Extract page type from the coordinator's unified perception."""
        perception = signal.unified_perception.lower()
        for pt in ["search_form", "search_results", "product_page", "checkout",
                    "login", "registration", "signup", "form", "cart",
                    "error", "article", "dashboard", "captcha", "paywall"]:
            if pt in perception:
                return pt
        return "unknown"

    @staticmethod
    def _sanitize_goal_id(goal: str) -> str:
        """Create a short stable goal ID from the goal text."""
        import hashlib
        return hashlib.sha256(goal.encode()).hexdigest()[:12]


def json_safe(obj: Any) -> str:
    """Convert an object to a safe JSON string for prompt use."""
    import json as _json
    try:
        return _json.dumps(obj, default=str)
    except Exception:
        return str(obj)[:1000]


# ── Social Login Detection ────────────────────────────────────────
# Reusable utility for identifying social login / third-party auth
# buttons by text content. Used by both the Vision target filter and
# the interactive element annotator.

_SOCIAL_LOGIN_PATTERNS = (
    "sign in with", "sign up with", "continue with", "login with",
    "log in with", "register with", "connect with",
)
_SOCIAL_PROVIDERS = (
    "google", "facebook", "github", "apple", "microsoft",
    "twitter", "linkedin",
)


def _is_social_login_element(element: dict) -> bool:
    """Detect social login buttons by text/label/aria-label content.

    Returns True for elements like 'Sign in with Google', 'Continue
    with Facebook', etc. Used to prevent the Vision Eye from forcing
    the planner to click social login and to annotate these elements
    with [SOCIAL_LOGIN] tags in the interactive element list.
    """
    text = (
        element.get("text", "") or element.get("label", "")
        or element.get("aria_label", "") or element.get("aria-label", "")
        or ""
    ).lower().strip()
    if not text:
        return False
    for pattern in _SOCIAL_LOGIN_PATTERNS:
        if pattern in text:
            for provider in _SOCIAL_PROVIDERS:
                if provider in text:
                    return True
    # Also catch icon-only buttons with provider name in class/selector
    selector = (element.get("selector", "") or "").lower()
    tag = element.get("tag", "")
    if tag in ("button", "a"):
        for provider in _SOCIAL_PROVIDERS:
            if provider in selector:
                return True
    return False


def _extract_vision_target(
    eye_reports: list[Any],
    dom_elements: list[dict],
) -> Any | None:
    """Extract the Vision Eye's highest-confidence element identification.

    Guardrails:
    1. Confidence threshold: only trust Vision when confidence >= 0.6.
       Below that, Vision is guessing — don't force the planner.
    2. Cross-eye validation: the element Vision identified MUST be
       visible AND enabled in the DOM. If Vision says "click button #7"
       but DOM says #7 is a disabled input, the target is rejected.
    """
    from ans_nerves.planner.planner import VisionConfirmedTarget

    _MIN_VISION_CONFIDENCE = 0.6

    for report in eye_reports:
        if getattr(report, "eye_name", "") != "vision":
            continue
        content = getattr(report, "content", {}) or {}
        visible = content.get("visible_elements", []) or []
        if not visible:
            continue
        best: dict | None = None
        best_conf = 0.0
        for ve in visible:
            ei = ve.get("element_index")
            if ei is None:
                continue
            conf = float(ve.get("confidence", 0.5))
            if conf > best_conf:
                best_conf = conf
                best = ve
        if not (best and best.get("element_index") is not None):
            continue
        if best_conf < _MIN_VISION_CONFIDENCE:
            continue  # Vision is guessing — don't force the planner

        ei = best["element_index"]
        sel = best.get("selector", "")
        label = best.get("label", "")

        # Cross-eye validation: match against DOM and verify the element
        # is actually interactive (visible + enabled).
        dom_match: dict | None = None
        for de in dom_elements:
            if de.get("element_index") == ei:
                dom_match = de
                break
        if dom_match is None:
            continue  # Vision identified an element not in the DOM list

        is_visible = dom_match.get("is_visible", True)
        is_enabled = dom_match.get("is_enabled", True)
        if not is_visible or not is_enabled:
            # DOM Reader says this element can't be interacted with.
            # Vision may have misread the SOM number or the element
            # is genuinely blocked/disabled. Don't force it.
            continue

        # Social login guard: never let Vision force the planner to
        # click a social login button. These are visually prominent
        # (large, colorful) and Vision often picks them as the
        # highest-confidence target, overriding the planner's own
        # reasoning about form fields.
        if _is_social_login_element(dom_match):
            logger.debug(
                "vision_target: skipping social login element '%s'",
                dom_match.get("text", "")[:60],
            )
            continue

        sel = dom_match.get("selector", sel)
        label = dom_match.get("label", label) or dom_match.get("text", label)

        return VisionConfirmedTarget(
            element_index=ei,
            selector=sel or "",
            label=label or "",
            confidence=best_conf,
        )
    return None
