---
name: roadmap-37-to-100
description: "Complete prioritized roadmap from current ~37% to 100% production-ready, with all gaps enumerated"
metadata:
  type: reference
  originSessionId: c30fcf07-e699-4d84-bedd-70bf1cdc0536
---

# Roadmap: 37% → 100%

## What's Complete (37%)

### Rust Skeleton (~85% of Rust done)
| Component | Status | Details |
|-----------|--------|---------|
| Layer 1 Gateway | Done | MCP 10 tools, REST 8 endpoints, WebSocket, all wired to real gRPC backend |
| Browser Control | Done | Chromium CDP, session management, daemon |
| DOM Distillation | Done | 3 modes (text_only, semantic, full), CSS class/id extraction |
| Page Diff | Done | Added/removed/modified detection, <500µs for no-change |
| Screenshot RPC | Done | Arrow IPC for bulk transfer |
| Signal Router | Done | 5-level contradiction resolution, relevance scoring, noise suppression |
| Immune System | Done | 15 substring + LazyLock<Regex> injection patterns |
| Auth | Done | API key registry, constant-time XOR compare, permission scoping |
| Budget Tracker | Done | 4-level circuit breaker (Normal/Conservative/Critical/Emergency) |
| Decision Store | Done | In-memory Vec<DecisionRecord>, cosine similarity search, 5 tests |
| gRPC Server | Done | 28 RPCs across 7 service areas |
| Hardening | Done | Zero unwraps, lint-clean workspace, 11 integration tests, 10 benchmarks |

### Python Intelligence (~5% of Python done)
| Component | Status | Details |
|-----------|--------|---------|
| Decision Scorer | Works | 3-layer temporal model: immediate*0.6 + short_term*0.3 + long_term*0.1 |
| Base Eye ABC | Works | EyeReport dataclass, observe() interface |
| DOM Reader Eye | STUB | Returns empty elements, no processing |
| Vision Model Eye | STUB | Returns page_type="unknown", empty visible_elements |
| Page Diff Eye | STUB | Returns empty added/removed/modified, summary="no_change" |
| Goal Verifier Eye | STUB | Returns sub_goal_advanced=False, confidence=0.0 |
| Error Detector Eye | STUB | Returns hardcoded failure_type="silent_fail" |
| Cross-Eye Coordinator | STUB | synthesize() returns empty RoutedSignal |
| Goal Decomposer | STUB | decompose() returns GoalSpec with zero sub_goals |
| gRPC Client | PLACEHOLDER | "Phase 0 placeholder — will be replaced..." |

---

## Priority 1: Wire LLM (GPT-4o-mini) — Unblocks Everything

**Current state:** No LLM call exists anywhere in the codebase. No OpenAI SDK, no Anthropic SDK.

**Tasks:**
1. Add `openai` Python dependency (GPT-4o-mini via OpenAI SDK)
2. Create `ans_nerves/llm/client.py` — shared LLM client with:
   - GPT-4o-mini model configuration
   - Structured output (JSON mode / function calling) for Eye reports
   - Retry with exponential backoff
   - Token budget tracking per request
   - Async HTTP via httpx/aiohttp
3. Create `ans_nerves/llm/prompts.py` — prompt templates for each Eye
4. Wire into at minimum Vision Eye and Goal Verifier Eye first
5. Integration test: real GPT-4o-mini call → EyeReport → Coordinator → Scorer

**Effort:** 2 days

---

## Priority 2: Implement the 5 Eyes (Blocked by Priority 1)

### Vision Model Eye (`nerves/ans_nerves/eyes/vision.py`)
- Send screenshot (base64) + distilled DOM to GPT-4o-mini
- Return: visible elements with positions, overlays, blocked regions, page type, anomalies
- Output format: structured JSON matching EyeReport schema

### DOM Reader Eye (`nerves/ans_nerves/eyes/dom_reader.py`)
- Process distilled DOM into structured element list
- Classify elements: interactive (clickable, input, select), semantic (headings, nav, main), noise (ads, trackers)
- Identify CSS classes/IDs for distraction classification

### Page Diff Eye (`nerves/ans_nerves/eyes/page_diff.py`)
- Use the Rust diff engine output (already available via gRPC)
- Send diff delta to GPT-4o-mini for semantic interpretation
- Return: what changed, why it matters, whether goal-advancing

### Goal Verifier Eye (`nerves/ans_nerves/eyes/goal_verifier.py`)
- Send current page state + sub-goal to GPT-4o-mini
- Return: sub_goal_advanced (bool), confidence (0-1), criteria_status, evidence from page
- This is the MOST IMPORTANT eye — it drives the agent's understanding of progress

### Error Detector Eye (`nerves/ans_nerves/eyes/error_detector.py`)
- Classify page state: error_page (404, 500, cloudflare, etc.), captcha, paywall, login_wall, normal
- Return: failure_type, recovery_actions (suggested next steps)

**Effort:** 3 days

---

## Priority 3: Intelligence Pipeline (Blocked by Priority 1+2)

### Cross-Eye Coordinator (`nerves/ans_nerves/coordinator/coordinator.py`)
- Take 5 EyeReports → synthesize into single RoutedSignal
- Implement 5-level contradiction resolution (defined in Rust signal router)
- Send conflicting reports to GPT-4o-mini for resolution
- Output: action + confidence + priority + routing hint

### Goal Decomposer (`nerves/ans_nerves/decomposer/decomposer.py`)
- Take goal_description + context → GPT-4o-mini → GoalSpec with sub_goals
- Each sub_goal: description, success_criteria, max_actions, expected_page_pattern
- Chain of thought: plan → decompose → validate

### Python gRPC Client (`nerves/ans_nerves/grpc_client.py`)
- Replace placeholder with real generated gRPC client
- Use `ans-proto` crate's .proto definitions
- Wire Python eyes → gRPC → Rust daemon (submit_eye_reports, query_decisions, etc.)
- This is the CRITICAL bridge between Python intelligence and Rust execution

**Effort:** 3 days

---

## Priority 4: Decision Intelligence Layer Completion

### LanceDB On-Disk Storage
- Replace in-memory `Vec<DecisionRecord>` with LanceDB
- Configure from `ans.toml` (`[storage.decisions]`)
- Schema: decision_id, action, context_json, embedding_vector, score, outcome, timestamp
- Keep cosine similarity search working with LanceDB vector index

### Embedding Generation
- Integrate sentence-transformers (all-MiniLM-L6-v2 per config, 384-dim)
- Generate embeddings for: action descriptions, page contexts, goal descriptions
- Store in LanceDB alongside decision records

### Feedback Loop End-to-End
- Action executed → outcome observed → score computed → stored with embedding
- Query best actions for similar context → influence next action selection
- This is the core differentiator vs. other agent-browser products

### Long-Term Outcome API
- `GET /api/v1/goals/{id}/outcomes` — business impact over days/months
- Connect to external analytics/webhooks for real business metrics
- Currently only immediate + short-term scoring exists

**Effort:** 3 days

---

## Priority 5: Production Hardening (from production-gaps.md)

### Gap 1: 610 Clippy Warnings
- 112 must_use, 84 docs missing backticks, 51 missing # Errors, 47 const fn, 26 PartialEq without Eq, ~40 integer casting
- **Effort:** 1 day

### Gap 2: Observability
- Add `metrics` crate: session count, goal count, actions/sec, CDP latency, injection detection rate, error rate
- Expose `GET /api/v1/metrics` (Prometheus scrape target)
- Structured log aggregation (tracing-loki)
- **Effort:** 1 day

### Gap 3: CI/CD Pipeline
- GitHub Actions: build, test, clippy, fmt on push/PR; bench on merge to main
- `.github/workflows/ci.yml`
- **Effort:** 0.5 day

### Gap 4: Containerization
- Dockerfile: Rust build → slim runtime + pinned Chromium
- docker-compose.yml for local dev
- **Effort:** 0.5 day

### Gap 5: Load Testing
- `ans-bench` crate: 10 concurrent sessions, memory profiling, CDP latency p50/p99
- **Effort:** 1 day

### Gap 6: Budget E2E Verification
- Integration test: create goal → execute actions → verify mode transitions
- **Effort:** 0.5 day

### Gap 7: Chrome Version Matrix
- Test Chrome stable/beta/dev on Windows/macOS/Linux
- CI matrix
- **Effort:** 1 day

---

## Priority 6: Dashboard + CLI (Plan Phase 8 — Never Built)

- Web dashboard: session viewer, goal tracker, budget monitor, immune alerts
- CLI tool: `ans` command for controlling daemon, viewing status, running goals
- Can be TypeScript (zero in critical path per plan)
- **Effort:** 3 days

---

## Summary: Effort and Dependency Chain

```
P1: Wire GPT-4o-mini (2d)
  └─> P2: Implement 5 Eyes (3d)
        └─> P3: Intelligence Pipeline (3d)
              └─> P4: Decision Intelligence (3d)
P5: Production Hardening (5.5d, independent)
P6: Dashboard + CLI (3d, independent)

Critical path: P1 → P2 → P3 → P4 = 11 days
Parallel track 1: P5 = 5.5 days
Parallel track 2: P6 = 3 days

Total effort: ~16 days (if sequential), ~11 days (with parallelism)
```

## Target LLM

User has chosen **GPT-4o-mini** via OpenAI SDK. Not Claude/Anthropic.
- Model: `gpt-4o-mini`
- SDK: `openai` Python package
- Mode: structured outputs (JSON mode) for Eye reports
- Cost: ~$0.15/1M input tokens, ~$0.60/1M output tokens

## Integration Pattern

The product integrates via **MCP** (already built and working):
- External AI agents connect through MCP (10 tools), REST API (8 endpoints), or WebSocket
- Internal Python intelligence connects via gRPC (to be wired)
- The Gateway (port 50052) is the single entry point
- The Daemon (port 50051) is the internal gRPC backend
