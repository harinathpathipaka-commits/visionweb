---
name: product-status-may-2026
description: "Full ANS Nerves product status as of 2026-05-28: Python (258 tests, 252 pass), Rust (15 crates, builds clean), CI green, Docker ready, all production gaps closed"
metadata: 
  node_type: memory
  type: project
  originSessionId: e34bc63b-503e-414b-8fec-720c371ef6a9
---

## Python Intelligence Layer (ans_nerves) — 32 source files, 258 tests (252 pass, 6 skip)

- **5 Eyes**: DOM Reader (deterministic), Vision (LLM), Page Diff (LLM), Goal Verifier (LLM), Error Detector (LLM)
- **Cross-Eye Coordinator**: Synthesizes 5 reports, 5-level contradiction resolution
- **Goal Decomposer**: LLM-driven goal→SubGoal DAG
- **Agent Planner**: Cold-start (LLM) + warm-start (memory-validated LanceDB)
- **Decision Intelligence**: LanceDB with 1536-dim OpenAI embeddings, composite scorer
- **Agent Loop**: Decompose → plan → execute → verify → score → repeat
- **LLM Client**: OpenAI GPT-4o-mini with tenacity retry
- **gRPC Client**: Circuit breaker (3-state) + retry
- **CLI**: config, health, decompose, serve subcommands
- **Health**: /health + /ready endpoints

## Rust Daemon (crates/) — 15 crates, builds clean

- ans-cdp: Chromium launch, CDP WebSocket, all browser actions (click, type, scroll, select, navigate, submit, screenshot, execute_script, wait_for_load)
- ans-distill: 3 DOM distillation modes
- ans-diff: Before/after page comparison
- ans-goal: Goal tracking
- ans-ipc: gRPC server, SessionManager (concurrent sessions, goal-scoped)
- ans-gateway: MCP server (10 tools), REST (8 endpoints), WebSocket
- ans-signal: Signal router with contradiction resolution
- ans-immune: Injection defense, auth
- ans-budget: Credit/token tracking
- ans-storage: Persistence
- ans-daemon: Main orchestrator
- ans-proto: Protobuf definitions
- ans-core: Shared types + Prometheus metrics
- ans-bench: Criterion benchmarks
- ans-stealth: Anti-detection + humanized interaction

## Production Readiness — ALL 7 GAPS CLOSED

- Clippy: 64 warnings (from 610)
- Observability: Prometheus metrics at GET /api/v1/metrics
- CI/CD: Full pipeline across 3 OSes + Chrome matrix
- Containerization: Dockerfile + docker-compose.yml
- Chrome matrix: stable + beta in CI
- Budget E2E: ans-budget tested
- Benchmarks: ans-bench compile-verified in CI

## Known Limitations (2026-05-28)

- No multi-tab within a session (each session = one Chromium page; Target.* domain not wired)
- 6 Python tests skipped (likely require OPENAI_API_KEY)
- 64 clippy warnings (cosmetic, not errors)
- Concurrent session load testing not yet implemented

**Why:** Updated after verification that all gaps are closed and CI/docker/metrics are in place.
**How to apply:** Product is test-ready. Reference for E2E test design and remaining limitations.
