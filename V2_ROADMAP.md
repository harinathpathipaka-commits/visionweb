# V2+ Roadmap — Deferred Features & Architecture

Everything intentionally removed from v1 to keep the 24-week critical path. Each item has a clear trigger condition for when it should be built.

---

## 1. WASM ML Immune System

**Status in v1:** Heuristic-only distraction classifier (regex, CSS patterns, ARIA inspection, URL matching). 95%+ coverage on known distraction types. No ML dependency.

**What v2 adds:**
- WASM-compiled ML model (ONNX runtime or burn.rs) loaded into the Rust immune pipeline
- Trained on labeled distraction samples collected from v1 heuristic decisions
- Replaces/supplements heuristic rules for edge cases the regex patterns miss
- Same `<10ms` latency budget, same `ImmuneClassifier` struct (field already reserved)

**Trigger:** Collect 50,000+ labeled (page_content, distraction_type) samples from v1 heuristic classifier running in production. Heuristic labels are the training data. When model accuracy exceeds heuristic on held-out test set, ship WASM model.

**Crate:** `ans-immune` (same crate, feature-gated)

---

## 2. NATS Message Broker (Multi-Node)

**Status in v1:** Single-node only. `tokio::broadcast` for internal pub/sub. gRPC streaming for Python↔Rust event delivery. Zero external message broker.

**What v2 adds:**
- NATS JetStream for cross-node pub/sub
- Multiple daemon instances sharing goal state via NATS
- Distributed eye processing (Vision Eye runs on GPU node, DOM Eye on browser node)
- Transparent migration — all subscribers already consume via gRPC streaming interface. Only daemon internals change (`tokio::broadcast` → NATS client)

**Trigger:** Need to scale beyond a single machine. Either: (a) >50 concurrent browser sessions saturating one machine, or (b) GPU-heavy vision processing needs separate hardware.

**Architecture change:**
```
v1:  tokio::broadcast ──→ gRPC streaming ──→ Python subscribers
v2:  NATS JetStream ──→ gRPC streaming (multi-node) ──→ Python subscribers
```

---

## 3. Servo Browser Engine (Alternative Backend)

**Status in v1:** Chromium only via CDP. `BrowserBackend` trait exists but only `CdpBackend` implemented.

**What v2 adds:**
- `ServoBackend` implementing the same `BrowserBackend` trait
- Servo for lightweight pages (documentation, forms, search results, no heavy JS)
- Chromium reserved for complex JS-heavy sites
- Automatic engine selection based on page characteristics (or agent preference)
- Lower memory per session (Servo ~50MB vs Chromium ~200MB)

**Trigger:** 100+ concurrent sessions where Chromium memory becomes the bottleneck. Servo's embedding API matures (currently experimental).

**Crate:** `ans-servo` (new, feature-gated)

---

## 4. Firefox / WebKit Backend

**Status in v1:** Not started. `BrowserBackend` trait designed for it, but zero implementation.

**What v2 adds:**
- `FirefoxBackend` via Firefox's CDP-compatible remote debugging protocol
- `WebKitBackend` via WebKit's remote inspector protocol (for macOS Safari testing)
- Cross-browser testing scenarios (agent verifies site works in Chrome + Firefox + Safari)

**Trigger:** Enterprise requirement for cross-browser testing. Or CDP protocol churn makes Chromium-only risky (mitigation in risk matrix).

---

## 5. Dashboard (Developer UI)

**Status in v1:** Not built. Explicitly marked "optional, non-critical" in the architecture. Zero impact on critical path.

**What v2 adds:**
- **Live Session View:** Real-time display of agent sessions — what page, what goal, what action just executed, what the eyes see
- **Injection Alert Dashboard:** Every prompt injection detection with source page, flagged content, action taken (sanitized/blocked), severity score. Red-team audit trail
- **Decision Timeline:** Chronological view of every action scored across all 3 business outcome layers (Immediate/Short-Term/Long-Term). See which actions advance goals and which don't
- **Budget Monitor:** Per-goal and per-API-key spend tracking. Circuit breaker mode transitions visualized. Historical cost data
- **Session Replay:** Step-by-step replay of completed sessions with DOM snapshots, eye reports, and decision scores overlaid
- **Goal Analytics:** Aggregate stats — average actions per goal, success rate, most common failure patterns, cost per goal type

**Tech:** Next.js + gRPC-web via Envoy proxy → Rust daemon (read-only). Zero write path — dashboard cannot control agents, only observe.

**Trigger:** Need human visibility into agent behavior. First user complaint of "I don't know what my agent is doing." Dashboard is the answer.

---

## 6. CLI Tools (`ans` CLI)

**Status in v1:** Not built. Phase 8 alongside dashboard.

**What v2 adds:**
```
ans session list          # List active sessions with goal, page, status
ans session inspect <id>  # Live view of one session (what it sees, what it did)
ans goal status <id>      # Progress, sub-goals completed, remaining
ans decisions query "..."  # Search scored decision history
ans immune alerts          # Recent distraction/injection events
ans budget status          # Per-goal and total spend
ans agent connect          # Connect external agent via MCP (one-shot setup)
ans config validate        # Validate ans.toml
```

**Trigger:** Power users who don't want a browser dashboard. DevOps/headless-server use case.

---

## 7. Debug Visualizer

**Status in v1:** Not built.

**What v2 adds:**
- Visual overlay showing what each eye perceives on a page
- DOM tree with distillation annotations (what was kept, what was removed, why)
- Page diff side-by-side with highlighted changes the Goal Verifier saw
- Immune system overlay showing elements classified as distractions + injection attempts
- Content boundary markers visualized on the rendered page

**Trigger:** Debugging why an agent made a wrong decision. "Show me what the agent saw" is the core question.

---

## 8. Multi-Tenant Auth & Team Features

**Status in v1:** API key auth only. Single machine, single user. `api_keys.json` file with scoped permissions.

**What v2 adds:**
- OAuth2 / OIDC authentication (Google, GitHub, enterprise SSO)
- Team workspaces — multiple users sharing agent sessions, decision history, budgets
- Role-based access: Admin (full control), Operator (run agents, view dashboard), Viewer (read-only dashboard)
- Per-user API key management with usage quotas
- Audit log — who ran what agent, when, what it cost

**Trigger:** Team adoption. First request for "my colleague needs to see my agent's sessions."

---

## 9. External Webhook/Callback System

**Status in v1:** Short-term and long-term business outcomes defined in the 3-layer model but no automated collection. Short/long-term outcomes updated manually or via LLM fallback evaluation.

**What v2 adds:**
- Webhook registration API — external systems register callbacks for goal outcome events
- Goal Verifier triggers webhooks on sub-goal completion
- External callback → LanceDB record update with real business outcome data
- Supported outcomes: CI/CD pass/fail, customer reply sentiment, ticket resolution, deployment status, revenue events
- Webhook signature verification (HMAC)

**Trigger:** Need real business outcome data instead of LLM-estimated outcomes. First integration with CI/CD, CRM, or support desk.

---

## 10. Custom Eye Plugin System

**Status in v1:** Eyes capped at 5 (DOM Reader, Vision Model, Page Diff, Goal Verifier, Error Detector). Hard architecture gate.

**What v2 adds:**
- Eye trait/interface for third-party eyes
- Plugin registry — eyes register capabilities and subscribe to event streams
- Community eyes: Accessibility Checker, SEO Analyzer, Performance Profiler, Legal/Compliance Scanner
- Eye marketplace in dashboard

**Trigger:** User demand for domain-specific eyes beyond the built-in 5. First external eye contributed.

---

## 11. Cloud Deployment (Managed Service)

**Status in v1:** Local daemon only. User runs `ans-daemon` on their machine. No cloud component.

**What v2 adds:**
- Managed cloud offering — hosted daemon with pooled Chromium instances
- Elastic scaling (auto-spawn browser instances as session count grows)
- Centralized decision history across all users (federated learning potential)
- Usage-based billing (per session-hour, per LLM token, per vision API call)
- SOC2 compliance, data residency controls

**Trigger:** Users who don't want to run infrastructure locally. Enterprise procurement requirement.

---

## 12. Advanced Browser Features

**Status in v1:** Basic CDP — navigate, click, type, screenshot, extract DOM. One Chromium instance per session.

**What v2 adds:**
- Network interception (block trackers, analytics, ads at CDP level before DOM distillation)
- Cookie/session persistence across browser restarts (maintain login state)
- File download handling
- iframe isolation (treat cross-origin iframes as separate security boundaries)
- Browser profile management (pre-configured profiles with extensions, cookies, preferences)
- Mobile device emulation (CDP device metrics override)
- Geolocation / timezone spoofing

**Trigger:** Agent needs to interact with authenticated services, download files, or test mobile views.

---

## 13. Decision Intelligence — Advanced Scoring

**Status in v1:** Cosine similarity vector search on (action, tool, context). 3-layer business outcome model. k-NN retrieval from LanceDB.

**What v2 adds:**
- Cross-goal pattern transfer — patterns learned from "book a flight" goal inform "book a hotel" goal
- Collaborative scoring — scores from one agent's decisions influence another agent facing similar context
- A/B decision testing — agent tries two different actions, compares outcomes, updates scoring
- Time-decay weighting — recent outcomes weighted higher than old outcomes
- Anomaly detection on decision patterns (flag when agent behavior changes unexpectedly)

**Trigger:** Decision store reaches 1M+ records. Cross-goal pattern transfer becomes feasible.

---

## 14. Extended Language & Framework Support

**Status in v1:** Rust daemon + Python intelligence workers. TypeScript dashboard (non-critical). gRPC + Arrow IPC.

**What v2 adds:**
- Go SDK for external agents (generate Go gRPC client from proto)
- Java/Kotlin SDK (JVM ecosystem)
- REST API parity with gRPC (all gRPC endpoints exposed via REST for non-gRPC clients)
- Webhook output connectors (Slack, PagerDuty, Datadog — agent outcomes surface to ops tools)

---

## Summary Timeline

| Priority | Item | When | Effort |
|----------|------|------|--------|
| P0 | WASM ML Immune System | After 50K labeled samples collected | 4-6 weeks |
| P0 | NATS Multi-Node | When >50 concurrent sessions needed | 3-4 weeks |
| P1 | Dashboard | First user visibility request | 4-6 weeks |
| P1 | CLI Tools | Alongside dashboard | 2-3 weeks |
| P1 | Webhook/Callback System | First external integration | 2-3 weeks |
| P2 | Servo Backend | 100+ session scale requirement | 6-8 weeks |
| P2 | Debug Visualizer | After dashboard | 3-4 weeks |
| P2 | Multi-Tenant Auth | First team adoption | 3-4 weeks |
| P2 | Custom Eye Plugins | First community contribution | 4-6 weeks |
| P3 | Firefox/WebKit Backend | Enterprise cross-browser requirement | 6-8 weeks |
| P3 | Cloud Deployment | Enterprise procurement demand | 12-16 weeks |
| P3 | Advanced Browser Features | Authenticated service interaction | 8-12 weeks |
| P3 | Advanced Decision Scoring | 1M+ decision records | 6-8 weeks |
| P3 | Extended SDKs | External adoption demand | 4-6 weeks |

---

## Architecture Gates (unchanged from v1)

These architectural decisions from v1 remain hard gates — they protect against scope creep:

- `BrowserBackend` trait exists but only `CdpBackend` is v1
- Eyes capped at 5 (no plugin system in v1)
- No WASM ML in immune system (heuristic-only)
- No NATS (single-node tokio::broadcast)
- TypeScript zero in critical path (dashboard/CLI/debugger only)
- gRPC + Arrow IPC remains the only Rust↔Python bridge
