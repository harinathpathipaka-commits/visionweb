"""The Five Eyes — goal-directed perception.

1. DOM Reader Eye  — Structured DOM perception
2. Vision Eye       — Screenshot-based visual understanding
3. Page Diff Eye    — Change detection between pages
4. Goal Verifier Eye — Sub-goal criteria checking
5. Error Detector Eye — Failure classification + recovery strategies
"""

from .base import BaseEye, EyeReport
from .dom_reader import DomReaderEye
from .vision import VisionEye
from .page_diff import PageDiffEye
from .goal_verifier import GoalVerifierEye
from .error_detector import ErrorDetectorEye

__all__ = [
    "BaseEye",
    "EyeReport",
    "DomReaderEye",
    "VisionEye",
    "PageDiffEye",
    "GoalVerifierEye",
    "ErrorDetectorEye",
]
