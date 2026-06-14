"""Embedding generator — FastEmbed (ONNX) primary, hash fallback.

FastEmbed: lightweight ONNX runtime, no HuggingFace symlink issues, instant startup.
Hash fallback: deterministic, zero-cost, always available.
"""

from __future__ import annotations

import hashlib
import struct
from typing import Any

import numpy as np

from ans_nerves.config import get_config
from ans_nerves.logging import get_logger

logger = get_logger(__name__)


class EmbeddingGenerator:
    """FastEmbed embeddings with deterministic hash fallback."""

    def __init__(self, model_name: str | None = None) -> None:
        config = get_config()
        self._model_name = model_name or config.llm.embedding_model
        self._dimension = config.llm.embedding_dim
        self._local_model: Any = None
        self._local_available: bool | None = None

    @property
    def dimension(self) -> int:
        return self._dimension

    @property
    def model_available(self) -> bool:
        """Lazy-load FastEmbed model on first use."""
        if self._local_available is None:
            try:
                from fastembed import TextEmbedding
                self._local_model = TextEmbedding(model_name=self._model_name)
                # Validate by running one trivial embedding
                next(self._local_model.embed(["test"]))
                self._local_available = True
                logger.info("FastEmbed loaded: %s (%d-dim)", self._model_name, self._dimension)
            except Exception as exc:
                self._local_available = False
                logger.info("FastEmbed unavailable (%s), using hash fallback", exc)
        return self._local_available

    async def embed_async(self, text: str) -> list[float]:
        if not text.strip():
            return [0.0] * self._dimension

        # 1. FastEmbed (ONNX, fast, free)
        if self.model_available and self._local_model is not None:
            try:
                import asyncio
                def _encode():
                    embeddings = list(self._local_model.embed([text]))
                    return embeddings[0].tolist() if len(embeddings) > 0 else []
                emb = await asyncio.to_thread(_encode)
                if emb:
                    return self._pad_or_truncate(emb, self._dimension)
            except Exception as exc:
                logger.debug("FastEmbed encode failed: %s", exc)

        # 2. Hash fallback (instant, deterministic)
        return self._fallback_embed(text)

    async def embed_batch_async(self, texts: list[str]) -> list[list[float]]:
        if not texts:
            return []

        if self.model_available and self._local_model is not None:
            try:
                import asyncio
                def _encode():
                    embeddings = list(self._local_model.embed(texts))
                    return [e.tolist() for e in embeddings]
                embs = await asyncio.to_thread(_encode)
                return [self._pad_or_truncate(e, self._dimension) for e in embs]
            except Exception as exc:
                logger.debug("FastEmbed batch failed: %s", exc)

        return [self._fallback_embed(t) for t in texts]

    def _fallback_embed(self, text: str) -> list[float]:
        tokens = text.lower().split()
        if not tokens:
            return [0.0] * self._dimension
        vec = np.zeros(self._dimension, dtype=np.float32)
        for token in set(tokens):
            h = hashlib.sha256(token.encode()).digest()
            for i in range(0, 32, 4):
                idx = struct.unpack("<I", h[i: i + 4])[0] % self._dimension
                vec[idx] += 1.0
        norm = float(np.linalg.norm(vec))
        if norm > 0:
            vec = vec / norm
        return vec.tolist()

    def build_context_text(
        self, action_type: str = "", selector: str = "",
        tool: str = "", goal_description: str = "", page_type: str = "",
    ) -> str:
        parts = [
            f"action:{action_type}", f"tool:{tool}", f"selector:{selector}",
            f"goal:{goal_description}", f"page:{page_type}",
        ]
        return " ".join(parts)

    @staticmethod
    def _pad_or_truncate(vec: list[float], target_dim: int) -> list[float]:
        if len(vec) == target_dim:
            return vec
        if len(vec) > target_dim:
            return vec[:target_dim]
        return vec + [0.0] * (target_dim - len(vec))


# Singleton
_generator: EmbeddingGenerator | None = None


def get_embedding_generator() -> EmbeddingGenerator:
    global _generator
    if _generator is None:
        _generator = EmbeddingGenerator()
    return _generator
