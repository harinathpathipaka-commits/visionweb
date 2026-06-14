---
name: quality-standards
description: "Production-grade infrastructure quality bar — no AI toys, no shortcuts. Every implementation phase must meet this standard."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 13a826f6-8425-4736-961c-8836f660c858
---

## Rule

Every phase implementation must be **rock solid and production grade**. We are building infrastructure, not an AI toy.

**Why:** The Agent Nervous System is a real product that external agents depend on. Crashes, race conditions, memory leaks, or sloppy error handling are unacceptable — this isn't a prototype or demo. The code must run reliably for hours/days as a long-running daemon.

**How to apply:**
- Every Rust struct/enum gets proper `Debug`, `Clone`, `Error` derives
- Every fallible operation returns `Result<T, E>` with proper error types (no `unwrap()` in production code without justification)
- Every async task has timeout handling and cancellation safety
- Memory: no unbounded collections, no leaked tasks, no leaked browser processes
- gRPC/Arrow IPC boundaries: defensive deserialization, version checks, schema validation
- Browser processes: guaranteed cleanup on drop (Chromium must not survive the daemon)
- Tests required for every crate before moving to next phase
- Error messages must be actionable ("CDP connection timeout after 30s for session abc123" not "Connection failed")
- Logging via `tracing` with structured spans (session_id, goal_id on every event)
- No TODO comments that ship to production — either fix it or mark it with a tracking issue

## Phase completion checklist

Before marking any phase complete, verify:
- [ ] `cargo build --workspace` passes with zero warnings
- [ ] `cargo test --workspace` passes all tests
- [ ] `cargo clippy --workspace` clean (no warnings)
- [ ] No unwrap() calls without documented invariants
- [ ] All public types have Debug
- [ ] All errors implement std::error::Error
- [ ] Graceful shutdown handles all cleanup
- [ ] README or crate-level docs explain what the crate does
