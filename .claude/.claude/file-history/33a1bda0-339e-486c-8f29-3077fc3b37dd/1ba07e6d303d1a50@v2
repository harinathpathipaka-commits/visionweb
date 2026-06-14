"""Async gRPC client for the Agent Nervous System Rust daemon.

Generated from proto/ans.proto. Connects to 127.0.0.1:50051 by default.
All methods are async via grpc.aio with tenacity retry + circuit breaker.
"""

from __future__ import annotations

from collections.abc import Awaitable, Callable
from typing import Any

import grpc
from google.protobuf.json_format import MessageToDict
from grpc import aio, StatusCode
from tenacity import (
    retry,
    stop_after_attempt,
    wait_exponential,
    retry_if_exception,
    RetryError,
)

from ans_nerves.ans_pb2 import (
    Action,
    ActionCheckRequest,
    BudgetConfigRequest,
    BudgetStatusRequest,
    CloseSessionRequest,
    CreateGoalRequest,
    CreateSessionRequest,
    DiffRequest,
    DistractionRequest,
    DomRequest,
    Empty,
    ExecuteActionRequest,
    GoalStateRequest,
    InjectionScanRequest,
    NavigateRequest,
    ProgressUpdate,
    QueryBestActionsRequest,
    ScreenshotRequest,
    SearchRequest,
    SessionRequest,
    StoreScoreRequest,
    SubmitReportsRequest,
)
from ans_nerves.ans_pb2_grpc import AgentNervousSystemStub
from ans_nerves.circuit_breaker import CircuitBreaker
from ans_nerves.config import get_config
from ans_nerves.exceptions import CircuitBreakerOpenError
from ans_nerves.logging import get_logger

logger = get_logger(__name__)

# gRPC status codes that are safe to retry
_RETRYABLE_CODES = frozenset({
    StatusCode.UNAVAILABLE,
    StatusCode.DEADLINE_EXCEEDED,
    StatusCode.RESOURCE_EXHAUSTED,
})

# gRPC status codes that should NOT trip the circuit breaker
_PERMANENT_CODES = frozenset({
    StatusCode.INVALID_ARGUMENT,
    StatusCode.NOT_FOUND,
    StatusCode.ALREADY_EXISTS,
    StatusCode.PERMISSION_DENIED,
    StatusCode.UNAUTHENTICATED,
    StatusCode.UNIMPLEMENTED,
})


def _is_retryable(exc: BaseException) -> bool:
    """Return True if the gRPC error is transient and worth retrying."""
    if isinstance(exc, aio.AioRpcError):
        return exc.code() in _RETRYABLE_CODES
    return False


def _should_trip_breaker(exc: BaseException) -> bool:
    """Return True if this error should count against the circuit breaker.

    Permanent errors (invalid args, not found, etc.) should NOT trip the breaker
    because they indicate a bug or misconfiguration, not a service outage.
    """
    if isinstance(exc, aio.AioRpcError):
        return exc.code() not in _PERMANENT_CODES
    return True


class GrpcClient:
    """Async gRPC client wrapping the generated AgentNervousSystemStub.

    Features:
    - Exponential backoff retry on transient errors (UNAVAILABLE, etc.)
    - Circuit breaker: opens after 5 consecutive failures, 30s recovery
    - Permanent errors skip retry and don't trip the breaker
    """

    def __init__(self, host: str | None = None, port: int | None = None) -> None:
        config = get_config()
        self._host = host or config.grpc_host
        self._port = port or config.grpc_port
        self._target = f"{self._host}:{self._port}"
        self._channel: aio.Channel | None = None
        self._stub: AgentNervousSystemStub | None = None
        self._breaker = CircuitBreaker(name=f"grpc:{self._target}")

    @property
    def target(self) -> str:
        return self._target

    @property
    def connected(self) -> bool:
        return self._channel is not None

    @property
    def breaker(self) -> CircuitBreaker:
        return self._breaker

    async def connect(self) -> None:
        """Open the gRPC channel to the Rust daemon."""
        if self._channel is not None:
            return
        self._channel = aio.insecure_channel(self._target)
        self._stub = AgentNervousSystemStub(self._channel)
        logger.info("grpc_client: connected to %s", self._target)

    async def close(self) -> None:
        """Close the gRPC channel."""
        if self._channel is not None:
            await self._channel.close()
            self._channel = None
            self._stub = None
            logger.info("grpc_client: disconnected")

    async def _ensure_connected(self) -> AgentNervousSystemStub:
        if self._stub is None:
            await self.connect()
        assert self._stub is not None
        return self._stub

    async def _call(
        self,
        coro_factory: Callable[[], Awaitable[Any]],
        method_name: str,
    ) -> Any:
        """Wrap a gRPC call with circuit breaker and exponential backoff retry.

        Args:
            coro_factory: An async callable that returns the gRPC result.
            method_name: Human-readable method name for log messages.
        """
        if self._breaker.is_open:
            raise CircuitBreakerOpenError(
                f"Circuit breaker is OPEN for {self._target}. "
                f"Method '{method_name}' rejected."
            )

        @retry(
            stop=stop_after_attempt(3),
            wait=wait_exponential(multiplier=1, min=0.5, max=10),
            retry=retry_if_exception(_is_retryable),
            reraise=True,
        )
        async def _with_retry() -> Any:
            return await coro_factory()

        try:
            result = await _with_retry()
        except RetryError as exc:
            # Retries exhausted — the wrapped exception is in exc.__cause__
            cause = exc.__cause__ or exc
            if _should_trip_breaker(cause):
                self._breaker.record_failure()
            logger.error(
                "grpc_client: %s failed after retries: %s", method_name, cause
            )
            raise cause from None
        except Exception as exc:
            if _should_trip_breaker(exc):
                self._breaker.record_failure()
            raise
        else:
            self._breaker.record_success()
            return result

    @staticmethod
    def _pb_to_dict(pb_msg: Any) -> Any:
        """Convert a protobuf message to a plain dict, keeping scalars as-is."""
        if pb_msg is None:
            return None
        try:
            return MessageToDict(pb_msg, preserving_proto_field_name=True)
        except Exception:
            return pb_msg

    # ── Session Management ────────────────────────────────

    async def create_session(
        self,
        goal_id: str = "",
        start_url: str = "",
        headers: dict[str, str] | None = None,
    ) -> Any:
        async def _rpc() -> Any:
            stub = await self._ensure_connected()
            return await stub.CreateSession(
                CreateSessionRequest(
                    goal_id=goal_id,
                    start_url=start_url,
                    headers=headers or {},
                )
            )
        return await self._call(_rpc, "create_session")

    async def close_session(self, session_id: str, reason: str = "") -> Any:
        async def _rpc() -> Any:
            stub = await self._ensure_connected()
            return await stub.CloseSession(
                CloseSessionRequest(session_id=session_id, reason=reason)
            )
        return await self._call(_rpc, "close_session")

    async def navigate(
        self, session_id: str, url: str, timeout_ms: int = 30000
    ) -> Any:
        async def _rpc() -> Any:
            stub = await self._ensure_connected()
            return await stub.Navigate(
                NavigateRequest(session_id=session_id, url=url, timeout_ms=timeout_ms)
            )
        result = await self._call(_rpc, "navigate")
        return self._pb_to_dict(result)

    async def execute_action(
        self,
        session_id: str,
        action_type: str,
        selector: str = "",
        value: str = "",
        params: dict[str, str] | None = None,
    ) -> Any:
        async def _rpc() -> Any:
            stub = await self._ensure_connected()
            action = Action(
                action_type=action_type,
                selector=selector,
                value=value,
                params=params or {},
            )
            return await stub.ExecuteAction(
                ExecuteActionRequest(session_id=session_id, action=action)
            )
        result = await self._call(_rpc, "execute_action")
        return self._pb_to_dict(result)

    async def evaluate(self, session_id: str, script: str) -> Any:
        """Execute arbitrary JavaScript in the page and return the result."""
        return await self.execute_action(
            session_id,
            action_type="evaluate",
            value=script,
        )

    # ── Perception ─────────────────────────────────────────

    async def get_distilled_dom(self, session_id: str, mode: str = "all_fields") -> Any:
        async def _rpc() -> Any:
            stub = await self._ensure_connected()
            return await stub.GetDistilledDom(
                DomRequest(session_id=session_id, mode=mode)
            )
        result = await self._call(_rpc, "get_distilled_dom")
        return self._pb_to_dict(result)

    async def capture_screenshot(
        self,
        session_id: str,
        fmt: str = "png",
        quality: int = 80,
        full_page: bool = False,
    ) -> Any:
        async def _rpc() -> Any:
            stub = await self._ensure_connected()
            return await stub.CaptureScreenshot(
                ScreenshotRequest(
                    session_id=session_id,
                    format=fmt,
                    quality=quality,
                    full_page=full_page,
                )
            )
        result = await self._call(_rpc, "capture_screenshot")
        return self._pb_to_dict(result)

    async def compute_diff(self, session_id: str, before: Any, after: Any) -> Any:
        async def _rpc() -> Any:
            stub = await self._ensure_connected()
            return await stub.ComputeDiff(
                DiffRequest(session_id=session_id, before=before, after=after)
            )
        result = await self._call(_rpc, "compute_diff")
        return self._pb_to_dict(result)

    # ── Eye Reports ────────────────────────────────────────

    async def submit_eye_reports(
        self, session_id: str, reports: list[Any]
    ) -> Any:
        async def _rpc() -> Any:
            stub = await self._ensure_connected()
            return await stub.SubmitEyeReports(
                SubmitReportsRequest(session_id=session_id, reports=reports)
            )
        return await self._call(_rpc, "submit_eye_reports")

    # ── Goal State ─────────────────────────────────────────

    async def create_goal(
        self,
        description: str,
        context: dict[str, str] | None = None,
        max_budget_cents: int = 500,
        max_steps: int = 50,
    ) -> Any:
        async def _rpc() -> Any:
            stub = await self._ensure_connected()
            return await stub.CreateGoal(
                CreateGoalRequest(
                    description=description,
                    context=context or {},
                    max_budget_cents=max_budget_cents,
                    max_steps=max_steps,
                )
            )
        result = await self._call(_rpc, "create_goal")
        return self._pb_to_dict(result)

    async def get_goal_state(self, goal_id: str) -> Any:
        async def _rpc() -> Any:
            stub = await self._ensure_connected()
            return await stub.GetGoalState(GoalStateRequest(goal_id=goal_id))
        return await self._call(_rpc, "get_goal_state")

    async def update_goal_progress(
        self,
        goal_id: str,
        progress: float,
        status: str = "",
        sub_goals: list[dict] | None = None,
    ) -> Any:
        async def _rpc() -> Any:
            stub = await self._ensure_connected()
            pb_sub_goals = []
            if sub_goals:
                from ans_nerves.ans_pb2 import SubGoal as PbSubGoal
                for sg in sub_goals:
                    pb_sub_goals.append(PbSubGoal(
                        id=sg.get("id", ""),
                        description=sg.get("description", ""),
                        success_criteria=sg.get("success_criteria", []),
                        depends_on=sg.get("depends_on", []),
                        status=sg.get("status", "pending"),
                    ))
            return await stub.UpdateGoalProgress(
                ProgressUpdate(
                    goal_id=goal_id, progress=progress,
                    status=status, sub_goals=pb_sub_goals,
                )
            )
        return await self._call(_rpc, "update_goal_progress")

    # ── Immune System ──────────────────────────────────────

    async def classify_distractions(self, session_id: str, url: str, dom: Any) -> Any:
        async def _rpc() -> Any:
            stub = await self._ensure_connected()
            return await stub.ClassifyDistractions(
                DistractionRequest(session_id=session_id, url=url, dom=dom)
            )
        return await self._call(_rpc, "classify_distractions")

    async def scan_injections(
        self, session_id: str, url: str, page_content: str
    ) -> Any:
        async def _rpc() -> Any:
            stub = await self._ensure_connected()
            return await stub.ScanInjections(
                InjectionScanRequest(
                    session_id=session_id, url=url, page_content=page_content
                )
            )
        return await self._call(_rpc, "scan_injections")

    async def check_action(self, session_id: str, action: Action) -> Any:
        async def _rpc() -> Any:
            stub = await self._ensure_connected()
            return await stub.CheckAction(
                ActionCheckRequest(session_id=session_id, action=action)
            )
        return await self._call(_rpc, "check_action")

    # ── Decision Intelligence ──────────────────────────────

    async def store_score(
        self,
        session_id: str,
        goal_id: str,
        action: Action,
        tool: str,
        context_embedding: list[float] | None = None,
        outcome_score: float = 0.0,
        result_score: float = 0.0,
        error_message: str = "",
        error_penalty: float = 0.0,
        business_outcome: float = 0.0,
        context_type: str = "",
    ) -> Any:
        async def _rpc() -> Any:
            stub = await self._ensure_connected()
            return await stub.StoreScore(
                StoreScoreRequest(
                    session_id=session_id,
                    goal_id=goal_id,
                    action=action,
                    tool=tool,
                    context_embedding=context_embedding or [],
                    outcome_score=outcome_score,
                    result_score=result_score,
                    error_message=error_message,
                    error_penalty=error_penalty,
                    business_outcome=business_outcome,
                    context_type=context_type,
                )
            )
        return await self._call(_rpc, "store_score")

    async def query_best_actions(
        self,
        context_embedding: list[float],
        k: int = 5,
        min_score: float = 0.3,
        context_type: str = "",
    ) -> Any:
        async def _rpc() -> Any:
            stub = await self._ensure_connected()
            return await stub.QueryBestActions(
                QueryBestActionsRequest(
                    context_embedding=context_embedding,
                    k=k,
                    min_score=min_score,
                    context_type=context_type,
                )
            )
        return await self._call(_rpc, "query_best_actions")

    async def search_similar_decisions(
        self,
        context_embedding: list[float],
        k: int = 5,
        min_score: float = 0.3,
        context_type: str = "",
    ) -> Any:
        async def _rpc() -> Any:
            stub = await self._ensure_connected()
            return await stub.SearchSimilarDecisions(
                SearchRequest(
                    context_embedding=context_embedding,
                    k=k,
                    min_score=min_score,
                    context_type=context_type,
                )
            )
        return await self._call(_rpc, "search_similar_decisions")

    # ── Budget ─────────────────────────────────────────────

    async def get_budget_status(self, goal_id: str = "") -> Any:
        async def _rpc() -> Any:
            stub = await self._ensure_connected()
            return await stub.GetBudgetStatus(BudgetStatusRequest(goal_id=goal_id))
        return await self._call(_rpc, "get_budget_status")

    async def configure_budget(
        self,
        default_per_goal_cents: int = 500,
        daily_limit_cents: int = 5000,
        normal_pct: float = 1.0,
        conservative_pct: float = 0.5,
        critical_pct: float = 0.15,
    ) -> Any:
        async def _rpc() -> Any:
            stub = await self._ensure_connected()
            return await stub.ConfigureBudget(
                BudgetConfigRequest(
                    default_per_goal_cents=default_per_goal_cents,
                    daily_api_key_spend_limit_cents=daily_limit_cents,
                    normal_threshold_pct=normal_pct,
                    conservative_threshold_pct=conservative_pct,
                    critical_threshold_pct=critical_pct,
                )
            )
        return await self._call(_rpc, "configure_budget")

    # ── Health ─────────────────────────────────────────────

    async def health(self) -> Any:
        async def _rpc() -> Any:
            stub = await self._ensure_connected()
            return await stub.Health(Empty())
        return await self._call(_rpc, "health")


# Singleton — shared by coordinator, decomposer, and future decision layer
_client: GrpcClient | None = None


def get_grpc_client() -> GrpcClient:
    global _client
    if _client is None:
        _client = GrpcClient()
    return _client
