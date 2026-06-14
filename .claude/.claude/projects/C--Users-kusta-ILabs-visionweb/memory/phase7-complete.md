---
name: phase7-complete
description: "Phase 7 complete — Signal Router with relevance scoring, noise suppression, contradiction resolution. 10 new tests, CI green."
metadata: 
  node_type: memory
  type: project
  originSessionId: 13a826f6-8425-4736-961c-8836f660c858
---

# Phase 7 Complete: Signal Router & Daemon Integration

Signal routing layer done: Cross-eye contradiction resolution, relevance scoring, noise suppression, and unified perception synthesis. Wired into the gRPC `submit_eye_reports` RPC.

## What was built

### ContradictionResolver (`ans-signal/src/contradiction.rs`)
- 5-level authority hierarchy: DOM Reader (5) > Page Diff (4) > Goal Verifier (3) > Vision (2) > Error Detector (1)
- Cross-eye conflict detection: DOM vs Vision anomalies, Diff vs Goal Verifier on static pages, Error overrides
- Resolution notes formatted as human-readable contradiction strings
- 5 tests

### SignalRouter (`ans-signal/src/router.rs`)
- 7-stage pipeline: score → filter noise → resolve contradictions → synthesize perception → compute confidence → generate hint → collect alerts
- Relevance scoring: weighted blend of eye authority (30%), confidence (25%), content signals (15%), goal alignment (30%)
- Noise suppression: reports below 0.15 relevance threshold are filtered; fallback keeps highest-relevance report
- Content-specific boosting: error reports always 0.9, large diffs 0.8, goal advancement 0.8×confidence
- Action hint generation: contradictions → recovery hints, errors → retry, high goal relevance → prioritize
- Alert collection from vision anomalies, overlays, and error reports
- 5 tests

### Daemon Integration
- `ans-ipc` depends on `ans-signal`
- `IpcServer` and `AgentNervousSystemServer` both carry `SignalRouter`
- `submit_eye_reports` RPC: proto reports → core conversion → `SignalRouter::route()` → proto response
- Proto-to-core conversion function maps all 5 eye content variants

## Verification
- `cargo build --workspace` — 0 errors
- `cargo clippy --workspace --all-targets` — 0 new warnings
- `cargo fmt --all -- --check` — passes
- `cargo test --workspace` — 74 tests pass (10 new in ans-signal)

## Files changed
| File | Action |
|------|--------|
| `crates/ans-signal/src/contradiction.rs` | Rewrite from empty stub (~130 lines) |
| `crates/ans-signal/src/router.rs` | Rewrite from empty stub (~390 lines) |
| `crates/ans-signal/src/lib.rs` | Export ContradictionResolver |
| `crates/ans-signal/Cargo.toml` | Add chrono dep |
| `crates/ans-ipc/Cargo.toml` | Add ans-signal dep |
| `crates/ans-ipc/src/server.rs` | Add SignalRouter to IpcServer, rewrite submit_eye_reports, add proto→core conversion |
