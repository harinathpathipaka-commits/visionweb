---
name: openai-embedding-switch-complete
description: "OpenAI text-embedding-3-small replaced local SentenceTransformer; 252 tests pass, 0 failures"
metadata: 
  node_type: memory
  type: project
  originSessionId: 5d717524-6e7c-413b-b5ce-0144db49d4b7
---

OpenAI text-embedding-3-small (1536-dim) replaced local all-MiniLM-L6-v2. Config-driven via LLMConfig.embedding_model/embedding_dim. LanceDB schema built lazily from config. Stale 384-dim LanceDB databases cleared; tests use tempdir isolation. Full suite: 252 passed, 6 skipped (live LLM tests), 0 failures as of 2026-05-25.

**Why:** User requested OpenAI embeddings instead of local model, and wanted all test fixes verified.

**How to apply:** When touching embeddings, store, or intelligence layer — dim is 1536, `_embedding_dim()` resolves from config, tests must use tempdir stores.
