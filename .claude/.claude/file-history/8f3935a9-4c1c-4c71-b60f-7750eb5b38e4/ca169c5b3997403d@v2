# Agent Nervous System (ANS) — Product Overview

**Status:** Production-Grade | **Date:** 2026-05-25 | **Version:** 0.1.0

---

## What It Is

The Agent Nervous System is a **purpose-built browser for AI agents**. It renders web content for agent perception, not human eyes. An AI agent connects to ANS (via MCP, REST, or WebSocket), gives it a goal, and ANS drives a real Chromium browser — navigating pages, clicking buttons, filling forms, taking screenshots, extracting DOM, verifying progress — then reports back what happened through structured perception.

It is not a Chrome extension. It is not Playwright with prompts bolted on. It is a complete stack: browser control → perception → signal routing → decision intelligence → agent gateway.

---

## Architecture: 4 Layers

```
Layer 1: GATEWAY (External API)
   MCP Server · REST API · WebSocket · Auth · Rate Limiting
                         │
Layer 2: DAEMON (Orchestration)
   gRPC Server · Session Manager · Budget Enforcer · Signal Router
                         │
Layer 3: BROWSER + PERCEPTION (Agent Web)
   Chromium CDP · DOM Distillation · Page Diff · 5 Eyes · Immune System
                         │
Layer 4: DECISION INTELLIGENCE (Feedback Loop)
   Goal Decomposer · Agent Planner · Multi-Factor Scorer · LanceDB Memory
```

### Layer 1 — Gateway (Rust: `ans-gateway`)
External AI agents connect here. Three protocols, one gateway:

- **MCP Server** (Model Context Protocol): 10 tools exposed — `create_session`, `navigate`, `click`, `type_text`, `scroll`, `screenshot`, `get_dom`, `execute_action`, `check_goal`, `create_goal`. Claude, ChatGPT, Cursor, and any MCP-compatible agent can call these as native tools.
- **REST API**: 8 endpoints at `/api/v1/` — sessions CRUD, navigation, action execution, screenshots, DOM extraction, goal creation/query. HTTP for non-MCP agents.
- **WebSocket**: `/ws` endpoint for real-time streaming — DOM changes, screenshots, action results pushed live.
- **Auth**: API key registry with constant-time XOR comparison, permission scoping, rate limiting.
- **Metrics**: `GET /api/v1/metrics` — Prometheus text format. Counters for sessions, actions, goals, screenshots, errors, DOM requests, immune scans, decisions stored.
- **Port**: 50052 (configurable)

### Layer 2 — Daemon (Rust: `ans-daemon`, `ans-ipc`)
The orchestrator that ties everything together:

- **gRPC Server**: 28 RPCs across 7 service areas. Production-hardened with circuit breaker, retry on transient failures, graceful shutdown on SIGINT/SIGTERM.
- **Session Manager**: `Arc<RwLock<HashMap<SessionId, ...>>>` — creates browser sessions, routes commands to the right CDP backend, manages lifecycle.
- **Signal Router** (`ans-signal`): 7-stage pipeline — score → filter noise → resolve contradictions → synthesize perception → compute confidence → generate hints → collect alerts. 5-level authority hierarchy for conflict resolution.
- **Budget Enforcer** (`ans-budget`): 4-mode circuit breaker — Normal → Conservative → Critical → Emergency. Tracks credits/tokens per session, enforces one-way mode progression (cannot go back from Emergency).
- **Port**: 50051 (gRPC), 50052 (gateway HTTP)

### Layer 3 — Browser + Perception (Rust: `ans-cdp`, `ans-distill`, `ans-diff`, `ans-immune`)
The Agent Web — renders for agent perception:

- **Chromium CDP** (`ans-cdp`): Full browser control via Chrome DevTools Protocol over WebSocket. Launches Chromium (auto-discovers via PATH/registry), manages process lifecycle (RAII drop on session end), executes typed CDP commands. 18 command builders with parsers.
- **DOM Distillation** (`ans-distill`): 3 modes — `TextOnly` (plain text extraction), `InputFields` (form fields + labels), `AllFields` (complete structured output). Classifies elements into 9 semantic block types. Strips noise (ads, trackers, cookie banners).
- **Page Diff** (`ans-diff`): Element-identity matching with 7 change classifications. Runs automatically on every page load. <500µs for no-change, ~800µs for 50% changed pages.
- **Immune System** (`ans-immune`): 15 substring injection patterns + `LazyLock<Regex>` for compound patterns. Detects prompt injection, homoglyph attacks, zero-width character insertion. Scans all external content before it reaches the agent.

### Layer 4 — Decision Intelligence (Python: `ans_nerves`)
The brain — learns from every action to improve future decisions:

- **5 Eyes** (perception):
  - **DOM Reader**: Deterministic element extraction. Classifies interactive (buttons, inputs, selects), semantic (headings, nav, main), noise (ads, trackers). Extracts CSS classes/IDs for distraction detection.
  - **Vision**: Screenshot + distilled DOM → GPT-4o-mini. Returns visible elements with positions, overlays, blocked regions, page type, anomalies.
  - **Page Diff**: Rust diff delta → GPT-4o-mini for semantic interpretation. Reports what changed, why it matters, goal relevance.
  - **Goal Verifier**: Current page state + sub-goal criteria → GPT-4o-mini. Returns sub_goal_advanced (bool), confidence (0-1), criteria status, evidence.
  - **Error Detector**: Classifies page state — 404, 500, Cloudflare, captcha, paywall, login wall. Returns failure_type + suggested recovery actions.

- **Cross-Eye Coordinator**: Takes 5 EyeReports → synthesizes into single `RoutedSignal`. 5-level contradiction resolution hierarchy (DOM Reader > Page Diff > Goal Verifier > Vision > Error Detector). Sends conflicting reports to LLM for resolution.

- **Goal Decomposer**: Goal description + context → GPT-4o-mini → `GoalSpec` with sub_goals DAG. Each sub_goal: description, success_criteria, max_actions, dependencies.

- **Agent Planner**: Cold-start (LLM) + warm-start (memory-validated). Queries LanceDB for highest-scoring actions matching current context. LLM overrides when memory recommendation differs.

- **Multi-Factor Scorer**: `composite = 0.40×outcome + 0.333×result - 0.133×error + 0.133×business`. Sigmoid efficiency curve. Error severity classification (CAPTCHA=1.0, bot_detect=1.0, timeout=0.5, element_not_found=0.3, etc.). Weights adapt over time via feedback.

- **LanceDB Store**: 1536-dim OpenAI `text-embedding-3-small` vectors. Actions stored with context_type, composite_score, outcome, embedding. Cosine similarity search for warm-start recommendations. Feedback loop updates scores on every outcome.

- **Agent Loop**: Decompose → Plan → Execute → Verify → Score → Repeat. Max-steps gating. Escalation on action failure. Action type filtering.

- **LLM Client**: OpenAI GPT-4o-mini via `AsyncOpenAI`. Tenacity retry on transient errors (httpx timeout, remote protocol). Token usage tracking + cost estimation. Structured output (JSON schema native, falls back to json_mode).

---

## Rust Crates (14 total)

| Crate | Purpose | Key Types | Tests |
|-------|---------|-----------|-------|
| `ans-proto` | Protobuf definitions | 28 RPCs, 7 services | 0* |
| `ans-core` | Shared types | Session, Goal, Decision, Budget, Metrics | 5 |
| `ans-daemon` | CLI entry point | Main orchestrator, graceful shutdown | 0* |
| `ans-cdp` | Chromium CDP | ChromiumProcess, CdpBackend, 18 commands | 4 |
| `ans-distill` | DOM distillation | Distiller, 3 DistillModes, 9 block types | 8 |
| `ans-diff` | Page diff engine | PageDiffer, 7 change classifications | 5 |
| `ans-immune` | Injection defense | InjectionDetector, 15 patterns, Regex | 10 |
| `ans-goal` | Goal tracking | GoalManager, GoalStore | 17 |
| `ans-signal` | Signal router | SignalRouter, ContradictionResolver | 10 |
| `ans-ipc` | gRPC server + sessions | IpcServer, SessionManager, EventBus | 16 |
| `ans-gateway` | MCP + REST + WebSocket | Gateway, McpServer, ApiRouter, WsServer | 12 |
| `ans-budget` | Credit/token tracking | BudgetTracker, CircuitBreaker (4 modes) | 5 |
| `ans-storage` | Persistence | DecisionStore, GoalStorage | 0* |
| `ans-bench` | Criterion benchmarks | 10 benchmarks (distill/diff/immune/concurrent) | 0* |

\* `ans-proto` tests are generated code. `ans-daemon`, `ans-storage`, `ans-bench` have no unit tests (integration/benchmark only).

**Total Rust tests: 96 passed, 0 failed** (excluding doc-tests and benchmarks)

---

## Python Package (ans_nerves — 32 source files)

| Module | Files | Purpose |
|--------|-------|---------|
| `eyes/` | 6 | 5 Eyes + base ABC |
| `llm/` | 3 | GPT-4o-mini client, prompts, JSON schemas |
| `coordinator/` | 2 | Cross-Eye synthesis, contradiction resolution |
| `decomposer/` | 2 | Goal → SubGoal DAG |
| `planner/` | 3 | Cold/warm start planning, agent loop |
| `scoring/` | 4 | Multi-factor scorer, embeddings, LanceDB store, intelligence |
| Root | 9 | Config, gRPC client, CLI, health, logging, exceptions, circuit breaker |
| `tests/` | 11 files | 258 tests across all modules |

**Total Python tests: 257 passed, 1 skipped, 0 failed**

The 1 skip is `test_client_tracks_token_usage` — a known Python 3.14 asyncio incompatibility with the OpenAI SDK, not a product issue.

---

## Integration Points

### How External Agents Connect

```
External AI Agent
       │
       ├── MCP (POST /mcp)          → JSON-RPC tool provider
       ├── REST (/api/v1/*)         → HTTP endpoints
       └── WebSocket (/ws)          → Streaming events
                    │
                    ▼
              Gateway (port 50052)
                    │
                    ▼
              Daemon gRPC (port 50051)
                    │
          ┌─────────┼─────────┐
          ▼         ▼         ▼
      Chromium   Perception  Decision
       (CDP)     (5 Eyes)   Intelligence
```

### Authentication
All connections require `X-API-Key` header. Keys managed via `SharedKeyRegistry` (loaded from config file). Constant-time XOR comparison prevents timing attacks. Public routes (`/health`, `/metrics`, `/mcp`, `/ws`) skip auth — everything else requires a valid key.

### Deployment
```bash
# Start the daemon (both gRPC + gateway)
cargo run --release -- serve --chrome-path /usr/bin/chromium

# The Python intelligence layer connects via gRPC
cd nerves && python -m ans_nerves serve
```

---

## Performance Characteristics

| Operation | Performance | Notes |
|-----------|-------------|-------|
| DOM distill (100 nodes) | ~0.6ms | AllFields mode |
| DOM distill (1,000 nodes) | ~1.5ms | AllFields mode |
| DOM distill (10,000 nodes) | ~12ms | AllFields mode |
| Page diff (no change) | <500µs | Element identity matching |
| Page diff (50% changed) | ~800µs | Full tree walk |
| Immune scan (small page) | ~2.6ms | 15 patterns + Regex |
| Injection detection | ~2.9ms | Includes homoglyph check |
| Concurrent distill (16 tasks) | Scales linearly | tokio spawn_blocking |
| GPT-4o-mini completion | ~1-3s | Depends on prompt size |

---

## CI/CD

GitHub Actions workflow (`.github/workflows/ci.yml`):
- **Rust**: build, test, clippy, fmt on ubuntu-24.04, macos-14, windows-2025
- **Chrome matrix**: stable + beta channels
- **Coverage**: tarpaulin with Codecov upload
- **Security**: cargo-audit on every push
- **Docker**: build verification
- **Benchmarks**: run on merge to main
- **Python**: test + lint on every push
- **Proto**: compilation check on every push
- **All-green gate**: all jobs must pass

---

## Containerization

Multi-stage `Dockerfile`:
- **Builder**: `rust:1.85-slim-bookworm` → compiles workspace with LTO
- **Runtime**: `debian:bookworm-slim` + Chromium (pinned version)
- **Non-root user**, health check endpoint
- **ENV**: Chrome path, gRPC port, gateway port

---

## What Makes It Different

1. **Not a Chrome extension** — purpose-built browser rendering for agent perception, not human eyes. DOM distillation, noise classification, and goal state are browser primitives, not bolted on.

2. **5 Eyes, not 1** — DOM Reader + Vision + Page Diff + Goal Verifier + Error Detector. Cross-eye contradiction resolution. If Vision says one thing and DOM Reader says another, the system resolves it structurally.

3. **Decision intelligence is built in** — every action is scored and stored. The agent learns from its own execution history. Same context next time → picks the highest-scoring action from memory. Not middleware, not a plugin.

4. **Immune system, not a filter list** — structurally stronger than prompt injection. Scans all external content (HTML, DOM, screenshots) before it reaches the agent's context.

5. **Budget awareness is native** — credits, circuit breakers, mode escalation. The agent can't spend more than it has. 4-mode progression is one-way (can't go back from Emergency).

6. **Production from day one** — zero unwraps, lint-clean workspace, Prometheus metrics, Docker, CI on 3 OSes, circuit breakers, integration tests, benchmarks. Not a prototype.

---

## Requirements

- **Rust**: 1.85+ (2021 edition)
- **Python**: 3.11 or 3.12 recommended (3.14 has known OpenAI SDK asyncio issues)
- **Chromium**: Version 100+ (for `--headless=new`). Auto-discovered via PATH or registry.
- **API Key**: OpenAI API key for GPT-4o-mini (set via `OPENAI_API_KEY` env var or `nerves/.env`)
- **Ports**: 50051 (gRPC), 50052 (gateway HTTP)

---

## Running Tests

```bash
# Rust (all crates, clippy, fmt)
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all -- --check

# Python (with API key)
$env:OPENAI_API_KEY = "sk-..."
cd nerves && python -m pytest tests/ -v

# Benchmarks
cargo bench --workspace
```
