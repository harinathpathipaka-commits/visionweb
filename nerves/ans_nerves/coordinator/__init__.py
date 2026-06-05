"""Cross-Eye Coordinator — synthesizes all 5 eye reports into unified awareness.

Handles:
- Contradiction detection and resolution (e.g. DOM says button exists, Vision says it's covered)
- Situational awareness synthesis (natural language summary for Decision layer)
- Eye confidence tracking and weighting
"""

from .coordinator import CrossEyeCoordinator

__all__ = ["CrossEyeCoordinator"]
