---
name: decision-memory-gap
description: "Decision memory pipeline is split-brained: Python LanceDB works, Rust DecisionStore is always empty. gRPC sync never called."
metadata:
  type: project
  originSessionId: a40d33bf-19ab-407b-b281-8395d3c17198
---

## Decision Memory Pipeline — Current State (June 12, 2026)

### What Works
- Python side: actions ARE scored (6-dimension), stored in LanceDB on disk, queried for warm-start planning after 3+ records
- gRPC `store_score` method fully implemented on both Python client and Rust server
- Warm-start planner queries local LanceDB and passes recommendations to LLM prompt

### What's Broken
- **Python `record_action()` NEVER calls `grpc.store_score()`** — intelligence.py:133 stores locally, then returns. The gRPC sync call is missing.
- **Rust DecisionStore is always empty** — in-memory `Vec<DecisionRecord>`, populated only by gRPC which nobody calls
- **Rust has no LanceDB** — `ans-storage` crate has zero LanceDB dependency, uses brute-force Vec search
- **Dual disconnected stores** — Python LanceDB (on disk, persistent) vs Rust Vec (in memory, ephemeral)
- **Embeddings unverified** — FastEmbed may not be available in Docker container; hash fallback produces semantically meaningless vectors

### Fix Required (P0)
Add `await self._grpc.store_score(...)` in `nerves/ans_nerves/scoring/intelligence.py:record_action()` after the local store. This populates the Rust DecisionStore so external queries return real data.

### For "Perfect" Decision Memory
1. `crates/ans-storage/Cargo.toml` — add `lancedb` dependency
2. `crates/ans-storage/src/decisions.rs` — replace in-memory Vec with LanceDB
3. Verify FastEmbed (ONNX) loads in Docker container
4. Python and Rust should share the SAME LanceDB path, not two copies
5. GoalStore snapshot/restore (currently all no-ops) needs real LanceDB persistence

[[session-state-jun-2026]] [[decision-memory-pipeline]]
