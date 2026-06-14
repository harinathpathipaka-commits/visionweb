"""LanceDB persistent vector store for scored actions.

Stores (action, tool, context_embedding, scores) tuples with config-driven
embedding dimensions (default 384 via FastEmbed bge-small-en-v1.5).
Supports ANN similarity search for context-aware action selection and
long-term outcome updates.
"""

from __future__ import annotations

import time
from pathlib import Path
from typing import Any

import lancedb
import numpy as np
import pyarrow as pa

from ans_nerves.config import get_config
from ans_nerves.logging import get_logger

logger = get_logger(__name__)

_TABLE_NAME = "scored_actions"


def _embedding_dim() -> int:
    """Resolve embedding dimension from config (default 384 for bge-small-en-v1.5)."""
    return get_config().llm.embedding_dim


def _build_schema(dim: int) -> pa.Schema:
    """Build the LanceDB schema with the given embedding dimension."""
    return pa.schema([
        pa.field("id", pa.string()),
        pa.field("session_id", pa.string()),
        pa.field("goal_id", pa.string()),
        pa.field("action_type", pa.string()),
        pa.field("selector", pa.string()),
        pa.field("value", pa.string()),
        pa.field("tool", pa.string()),
        pa.field("context_type", pa.string()),
        pa.field("context_text", pa.string()),
        pa.field("vector", pa.list_(pa.float32(), dim)),
        pa.field("outcome_score", pa.float32()),
        pa.field("result_score", pa.float32()),
        pa.field("error_penalty", pa.float32()),
        pa.field("business_outcome", pa.float32()),
        pa.field("composite_score", pa.float32()),
        pa.field("use_count", pa.int32()),
        pa.field("last_used_at", pa.int64()),
        pa.field("error_message", pa.string()),
    ])


class LanceDBStore:
    """Persistent vector store for decision intelligence.

    Stores each (action, tool, context) tuple with its embedding and
    scores. Supports ANN similarity queries for context-aware retrieval.
    """

    def __init__(self, uri: str | None = None) -> None:
        config = get_config()
        self._uri = uri or str(config.data_dir / "decisions")
        self._db: lancedb.DBConnection | None = None
        self._table: lancedb.table.Table | None = None
        self._dim = _embedding_dim()

    @property
    def dim(self) -> int:
        return self._dim

    @property
    def db(self) -> lancedb.DBConnection:
        if self._db is None:
            Path(self._uri).mkdir(parents=True, exist_ok=True)
            self._db = lancedb.connect(self._uri)
            logger.info("LanceDB opened at %s", self._uri)
        return self._db

    @property
    def table(self) -> lancedb.table.Table:
        if self._table is None:
            try:
                self._table = self.db.open_table(_TABLE_NAME)
            except Exception:
                self._table = self.db.create_table(
                    _TABLE_NAME,
                    schema=_build_schema(self._dim),
                    mode="create",
                )
                logger.info("LanceDB table '%s' created (dim=%d)", _TABLE_NAME, self._dim)
        return self._table

    def store(
        self,
        *,
        session_id: str = "",
        goal_id: str = "",
        action_type: str = "",
        selector: str = "",
        value: str = "",
        tool: str = "",
        context_type: str = "",
        context_text: str = "",
        embedding: list[float] | None = None,
        outcome_score: float = 0.0,
        result_score: float = 0.0,
        error_penalty: float = 0.0,
        business_outcome: float = 0.0,
        composite_score: float = 0.0,
        error_message: str = "",
    ) -> str:
        """Store a scored action with its context embedding.

        Returns the record ID.
        """
        import uuid

        record_id = uuid.uuid4().hex[:16]
        now_ms = int(time.time() * 1000)
        vec = list(embedding) if embedding else [0.0] * self._dim

        # Pad or truncate to exact dimension
        if len(vec) < self._dim:
            vec.extend([0.0] * (self._dim - len(vec)))
        elif len(vec) > self._dim:
            vec = vec[:self._dim]

        composite = composite_score  # Use scorer-computed value, not hardcoded weights

        row: dict[str, Any] = {
            "id": record_id,
            "session_id": session_id or "",
            "goal_id": goal_id or "",
            "action_type": action_type,
            "selector": selector,
            "value": value,
            "tool": tool,
            "context_type": context_type,
            "context_text": context_text[:1000],
            "vector": vec,
            "outcome_score": float(outcome_score),
            "result_score": float(result_score),
            "error_penalty": float(error_penalty),
            "business_outcome": float(business_outcome),
            "composite_score": float(composite),
            "use_count": 1,
            "last_used_at": now_ms,
            "error_message": error_message[:500],
        }

        self.table.add([row])
        return record_id

    def query(
        self,
        embedding: list[float],
        k: int = 5,
        min_score: float = 0.0,
        context_type: str | None = None,
    ) -> list[dict[str, Any]]:
        """Find the best actions for a given context embedding via ANN search."""
        if not embedding or all(v == 0.0 for v in embedding):
            return self._fallback_query(k, min_score, context_type)

        vec = list(embedding)
        if len(vec) < self._dim:
            vec.extend([0.0] * (self._dim - len(vec)))

        try:
            results = (
                self.table.search(vec)
                .metric("cosine")
                .limit(k * 2)
                .to_list()
            )
        except Exception:
            logger.warning("LanceDB ANN search failed, using fallback")
            return self._fallback_query(k, min_score, context_type)

        scored: list[dict[str, Any]] = []
        for r in results:
            if r.get("composite_score", 0.0) < min_score:
                continue
            if context_type and r.get("context_type") != context_type:
                continue
            scored.append({
                "id": r.get("id", ""),
                "action_type": r.get("action_type", ""),
                "selector": r.get("selector", ""),
                "tool": r.get("tool", ""),
                "context_type": r.get("context_type", ""),
                "composite_score": r.get("composite_score", 0.0),
                "outcome_score": r.get("outcome_score", 0.0),
                "result_score": r.get("result_score", 0.0),
                "error_penalty": r.get("error_penalty", 0.0),
                "business_outcome": r.get("business_outcome", 0.0),
                "use_count": r.get("use_count", 0),
                "last_used_at": r.get("last_used_at", 0),
                "distance": r.get("_distance", 1.0),
            })

        scored.sort(key=lambda x: x["composite_score"], reverse=True)
        return scored[:k]

    def _all_rows(self) -> list[dict[str, Any]]:
        """Return all rows as a list of dicts using PyArrow (no pandas needed)."""
        try:
            arrow_table = self.table.to_arrow()
        except Exception:
            return []
        return arrow_table.to_pylist()

    def _fallback_query(
        self,
        k: int,
        min_score: float,
        context_type: str | None,
    ) -> list[dict[str, Any]]:
        """Return highest-scored actions when no embedding is available."""
        rows = self._all_rows()
        if not rows:
            return []

        if context_type:
            rows = [r for r in rows if r.get("context_type") == context_type]
        if min_score > 0:
            rows = [r for r in rows if r.get("composite_score", 0.0) >= min_score]

        rows.sort(key=lambda r: r.get("composite_score", 0.0), reverse=True)

        for r in rows[:k]:
            r["distance"] = 0.0
        return rows[:k]

    def update_outcome(
        self,
        record_id: str,
        *,
        business_outcome: float | None = None,
        result_score: float | None = None,
        error_penalty: float | None = None,
        increment_use: bool = False,
    ) -> bool:
        """Update long-term outcome fields on a stored record."""
        rows = self._all_rows()
        if not rows:
            return False

        updated = False
        for row in rows:
            if row.get("id") == record_id:
                if business_outcome is not None:
                    row["business_outcome"] = float(business_outcome)
                if result_score is not None:
                    row["result_score"] = float(result_score)
                if error_penalty is not None:
                    row["error_penalty"] = float(error_penalty)
                if increment_use:
                    row["use_count"] = int(row.get("use_count", 0)) + 1
                    row["last_used_at"] = int(time.time() * 1000)

                row["composite_score"] = (
                    float(row.get("outcome_score", 0.0)) * 0.40
                    + float(row.get("result_score", 0.0)) * 0.333
                    - float(row.get("error_penalty", 0.0)) * 0.133
                    + float(row.get("business_outcome", 0.0)) * 0.133
                )
                updated = True
                break

        if not updated:
            return False

        self._table = self.db.create_table(
            _TABLE_NAME,
            data=rows,
            mode="overwrite",
        )
        return True

    def count(self) -> int:
        try:
            return self.table.count_rows()
        except Exception:
            return 0


# Singleton
_store: LanceDBStore | None = None


def get_store() -> LanceDBStore:
    global _store
    if _store is None:
        _store = LanceDBStore()
    return _store
