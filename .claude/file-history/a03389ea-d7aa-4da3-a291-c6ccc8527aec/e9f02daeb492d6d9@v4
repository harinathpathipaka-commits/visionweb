"""Agent Nervous System — Python intelligence layer.

This package contains:
- llm/         — Shared LLM client, prompt templates for all Eyes
- eyes/        — The 5 Eyes: DOM Reader, Vision, Page Diff, Goal Verifier, Error Detector
- coordinator/ — Cross-Eye Coordinator with contradiction resolution
- decomposer/  — Goal Decomposer (LLM: text -> structured sub-goals with verifiable criteria)
- scoring/     — Decision Intelligence: scoring, embedding, LanceDB storage, feedback loop
- grpc_client/ — Async gRPC client for Rust daemon (21 RPCs)
- config.py    — Configuration from env vars and ans.toml
"""

from __future__ import annotations

import os
from pathlib import Path


def _load_dotenv() -> None:
    """Load .env file BEFORE any config module reads env vars."""
    candidates = [
        Path(__file__).resolve().parent.parent / ".env",  # nerves/.env
        Path.cwd() / ".env",
        Path.home() / ".ans" / ".env",
    ]
    for candidate in candidates:
        if not candidate.exists():
            continue
        try:
            with open(candidate, encoding="utf-8") as fh:
                for line in fh:
                    line = line.strip()
                    if not line or line.startswith("#") or "=" not in line:
                        continue
                    key, _, value = line.partition("=")
                    key = key.strip()
                    value = value.strip().strip("\"'")
                    if key and key not in os.environ:
                        os.environ[key] = value
            return  # first file wins
        except OSError:
            continue


_load_dotenv()

from ans_nerves.logging import configure_logging  # noqa: E402

configure_logging()

__version__ = "0.4.0"
