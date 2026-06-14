"""Custom exception hierarchy for ans-nerves."""

from __future__ import annotations


class NervesError(Exception):
    """Base exception for all ans-nerves errors."""


class GrpcConnectionError(NervesError):
    """gRPC connection failed or was refused."""


class GrpcTimeoutError(NervesError):
    """gRPC call exceeded deadline."""


class CircuitBreakerOpenError(NervesError):
    """Circuit breaker is open — too many recent failures."""


class ConfigValidationError(NervesError):
    """Configuration validation failed."""
