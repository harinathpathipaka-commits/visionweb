"""Base eye interface — every eye implements this."""

from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from datetime import datetime, timezone
from typing import Any


@dataclass
class EyeReport:
    """A single eye's observation with metadata."""

    eye_name: str
    timestamp: datetime = field(default_factory=lambda: datetime.now(timezone.utc))
    confidence: float = 1.0
    goal_relevance: float = 0.0
    content: dict[str, Any] = field(default_factory=dict)


class BaseEye(ABC):
    """Abstract eye — all five eyes implement this interface."""

    name: str = "base"

    @abstractmethod
    async def observe(self, session_id: str, page_data: dict[str, Any]) -> EyeReport:
        """Observe the current page state and produce a report."""
        ...
