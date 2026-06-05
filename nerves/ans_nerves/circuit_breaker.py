"""3-state circuit breaker for gRPC resilience.

CLOSED → (5 consecutive failures) → OPEN → (30s timeout) → HALF_OPEN
HALF_OPEN → (1 success) → CLOSED
HALF_OPEN → (1 failure) → OPEN
"""

from __future__ import annotations

import time
from enum import Enum

from ans_nerves.logging import get_logger

logger = get_logger(__name__)


class CircuitState(Enum):
    CLOSED = "closed"
    OPEN = "open"
    HALF_OPEN = "half_open"


class CircuitBreaker:
    """Simple 3-state circuit breaker.

    Opens after `failure_threshold` consecutive failures.
    Transitions to HALF_OPEN after `recovery_timeout` seconds.
    Closes on first success in HALF_OPEN.
    """

    def __init__(
        self,
        failure_threshold: int = 5,
        recovery_timeout: float = 30.0,
        name: str = "grpc",
    ) -> None:
        self._failure_threshold = failure_threshold
        self._recovery_timeout = recovery_timeout
        self._name = name
        self._state = CircuitState.CLOSED
        self._failure_count = 0
        self._last_failure_time = 0.0
        self._opened_at = 0.0

    @property
    def state(self) -> CircuitState:
        self._transition()
        return self._state

    @property
    def is_open(self) -> bool:
        return self.state == CircuitState.OPEN

    def _transition(self) -> None:
        if self._state == CircuitState.OPEN:
            if time.monotonic() - self._opened_at >= self._recovery_timeout:
                self._state = CircuitState.HALF_OPEN
                logger.info(
                    "circuit_breaker[%s]: OPEN → HALF_OPEN after %.1fs",
                    self._name,
                    self._recovery_timeout,
                )

    def record_success(self) -> None:
        self._failure_count = 0
        if self._state == CircuitState.HALF_OPEN:
            self._state = CircuitState.CLOSED
            logger.info("circuit_breaker[%s]: HALF_OPEN → CLOSED", self._name)
        self._last_failure_time = 0.0

    def record_failure(self) -> None:
        self._failure_count += 1
        self._last_failure_time = time.monotonic()

        if (
            self._state == CircuitState.CLOSED
            and self._failure_count >= self._failure_threshold
        ):
            self._state = CircuitState.OPEN
            self._opened_at = time.monotonic()
            logger.warning(
                "circuit_breaker[%s]: CLOSED → OPEN after %d consecutive failures",
                self._name,
                self._failure_count,
            )
        elif self._state == CircuitState.HALF_OPEN:
            self._state = CircuitState.OPEN
            self._opened_at = time.monotonic()
            logger.warning(
                "circuit_breaker[%s]: HALF_OPEN → OPEN (test call failed)",
                self._name,
            )
