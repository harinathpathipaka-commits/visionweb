"""Tests for gRPC client resilience features."""

import pytest
import grpc
from grpc import StatusCode

from ans_nerves.circuit_breaker import CircuitBreaker, CircuitState
from ans_nerves.exceptions import CircuitBreakerOpenError
from ans_nerves.grpc_client import _is_retryable, _should_trip_breaker


# ── Circuit Breaker ────────────────────────────────────────


class TestCircuitBreaker:
    def setup_method(self):
        self.cb = CircuitBreaker(failure_threshold=3, recovery_timeout=0.05, name="test")

    def test_starts_closed(self):
        assert self.cb.state == CircuitState.CLOSED
        assert not self.cb.is_open

    def test_opens_after_threshold(self):
        self.cb.record_failure()
        self.cb.record_failure()
        assert self.cb.state == CircuitState.CLOSED
        self.cb.record_failure()
        assert self.cb.state == CircuitState.OPEN
        assert self.cb.is_open

    def test_success_resets_counter(self):
        self.cb.record_failure()
        self.cb.record_failure()
        self.cb.record_success()
        self.cb.record_failure()
        self.cb.record_failure()
        assert self.cb.state == CircuitState.CLOSED  # only 2 consecutive

    def test_half_open_after_recovery(self):
        self.cb.record_failure()
        self.cb.record_failure()
        self.cb.record_failure()
        assert self.cb.state == CircuitState.OPEN
        import time
        time.sleep(0.1)
        assert self.cb.state == CircuitState.HALF_OPEN

    def test_half_open_success_closes(self):
        for _ in range(3):
            self.cb.record_failure()
        import time
        time.sleep(0.1)
        assert self.cb.state == CircuitState.HALF_OPEN
        self.cb.record_success()
        assert self.cb.state == CircuitState.CLOSED

    def test_half_open_failure_reopens(self):
        for _ in range(3):
            self.cb.record_failure()
        import time
        time.sleep(0.1)
        assert self.cb.state == CircuitState.HALF_OPEN
        self.cb.record_failure()
        assert self.cb.state == CircuitState.OPEN

    def test_custom_threshold(self):
        cb = CircuitBreaker(failure_threshold=5)
        for _ in range(4):
            cb.record_failure()
        assert cb.state == CircuitState.CLOSED
        cb.record_failure()
        assert cb.state == CircuitState.OPEN


# ── Retry predicates ───────────────────────────────────────


class TestRetryPredicates:
    def test_retryable_unavailable(self):
        exc = _make_grpc_error(StatusCode.UNAVAILABLE)
        assert _is_retryable(exc)

    def test_retryable_deadline_exceeded(self):
        exc = _make_grpc_error(StatusCode.DEADLINE_EXCEEDED)
        assert _is_retryable(exc)

    def test_retryable_resource_exhausted(self):
        exc = _make_grpc_error(StatusCode.RESOURCE_EXHAUSTED)
        assert _is_retryable(exc)

    def test_not_retryable_invalid_argument(self):
        exc = _make_grpc_error(StatusCode.INVALID_ARGUMENT)
        assert not _is_retryable(exc)

    def test_not_retryable_not_found(self):
        exc = _make_grpc_error(StatusCode.NOT_FOUND)
        assert not _is_retryable(exc)

    def test_not_retryable_permission_denied(self):
        exc = _make_grpc_error(StatusCode.PERMISSION_DENIED)
        assert not _is_retryable(exc)

    def test_not_retryable_non_grpc_error(self):
        assert not _is_retryable(ValueError("random"))


class TestShouldTripBreaker:
    def test_breakers_on_unavailable(self):
        exc = _make_grpc_error(StatusCode.UNAVAILABLE)
        assert _should_trip_breaker(exc)

    def test_no_breaker_on_invalid_argument(self):
        exc = _make_grpc_error(StatusCode.INVALID_ARGUMENT)
        assert not _should_trip_breaker(exc)

    def test_no_breaker_on_not_found(self):
        exc = _make_grpc_error(StatusCode.NOT_FOUND)
        assert not _should_trip_breaker(exc)

    def test_no_breaker_on_unauthenticated(self):
        exc = _make_grpc_error(StatusCode.UNAUTHENTICATED)
        assert not _should_trip_breaker(exc)

    def test_breaker_on_non_grpc_error(self):
        assert _should_trip_breaker(ValueError("random"))


class TestCircuitBreakerOpenError:
    def test_is_exception(self):
        err = CircuitBreakerOpenError("breaker is open")
        assert isinstance(err, Exception)
        assert "breaker is open" in str(err)


# ── Helpers ────────────────────────────────────────────────


def _make_grpc_error(code: StatusCode) -> grpc.aio.AioRpcError:
    """Create a fake AioRpcError with the given status code."""
    details = f"test error: {code.name}"
    # AioRpcError requires a grpc.aio.Call, which is hard to mock.
    # We create a minimal error via the non-aio path and wrap it.
    try:
        raise grpc.RpcError()
    except grpc.RpcError:
        pass

    # grpc.aio.AioRpcError(code, details) — direct construction
    return grpc.aio.AioRpcError(
        code,
        grpc.aio.Metadata(),  # initial_metadata
        grpc.aio.Metadata(),  # trailing_metadata
        details,
    )
