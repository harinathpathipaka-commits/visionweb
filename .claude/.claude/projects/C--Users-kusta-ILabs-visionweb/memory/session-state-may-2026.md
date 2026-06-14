---
name: session-state-may-2026
description: "Full ANS project state as of 2026-05-30 — all fixes, working pipeline, config, architecture decisions"
metadata: 
  node_type: memory
  type: project
  originSessionId: a03389ea-d7aa-4da3-a291-c6ccc8527aec
---

## ANS Current State — May 30, 2026

## What Works

### Full Pipeline (end-to-end tested)
- Chrome CDP launches and navigates to real websites ✓
- LLM Decomposer (DeepSeek V4-Flash) produces detailed sub-goals ✓
- Planner generates action sequences ✓
- Browser executes actions (navigate, click, type, scroll, get_dom, screenshot) ✓
- 5 Eyes (DOM Reader, Vision, Page Diff, Goal Verifier, Error Detector) verify every step ✓
- Cross-Eye Coordinator resolves contradictions ✓
- Decision Intelligence scores actions → embeds → stores in LanceDB ✓
- Error detection gating: 3 failures per sub-goal → escalation ✓
- AgentLoop completes and reports success/failure ✓

### MCP Integration
- Daemon starts on ports: gRPC 50051, Gateway 50052
- 10 MCP tools live: create_goal, check_goal, create_session, navigate, click, type_text, scroll, screenshot, get_dom, execute_action
- `create_goal` MCP tool spawns Python AgentLoop as background subprocess
- `check_goal` returns real progress (0% → 20% → 40% → 60% → 80% → complete)
- Agent integration: `{"mcpServers": {"ans": {"url": "http://127.0.0.1:50052/mcp"}}}`
- REST API at http://127.0.0.1:50052/api/v1/ — 8 endpoints
- WebSocket at ws://127.0.0.1:50052/ws

### Dashboard
- Live at http://127.0.0.1:50052/ — dark theme, glass morphism design
- Goal input bar, live progress cards, 5 Eyes panel, WebSocket connection status
- Fast/Thorough mode toggle
- Install scripts: install.ps1 (Windows), install.sh (Linux/macOS)

### Speed Optimizations
- Parallel Eyes (asyncio.gather) — 50s → 10s per cycle
- Skip Vision on clean pages (no overlays) — saved ~3s per cycle
- Vision Eye every 3rd cycle only (not every cycle)
- Decomposer response caching (hash goal → cached result, 0ms on repeat)
- CDP navigate timeout 10s (was 30s)
- Sub-goal cap at 5 (was unlimited, typical 7)
- Session reuse (no close after run, manual close())
- Current: ~2-3 min for first run, ~30s for cached repeat

### Architecture Decisions
- **Model split**: DeepSeek V4-Flash for text (Decomposer, Planner, Verifier, Error, Coordinator), GPT-4o for Vision (screenshots)
- **Prompt-only JSON**: No `response_format` API flags — works on any model
- **Venve auto-detection**: Daemon finds `.venv/Scripts/python.exe` automatically
- **Chromium**: Searches Playwright cache first, then PATH, then auto-download via `npx playwright install chromium`
- **Embeddings**: Local BGE-M3 (1024-dim) primary → OpenAI fallback → hash last-resort
- **Fast Mode**: `ANS_MODE=fast` env var skips Vision/PageDiff/Coordinator on cache hits
- **Progress**: Real-time incremental updates via gRPC `update_goal_progress`, visible in both MCP and dashboard

## Files Structure

```
C:\Users\kusta\ILabs\visionweb\
├── target/release/ans-daemon.exe    # Built binary (latest)
├── ans.toml                          # Daemon config
├── ANS_ROADMAP.md                    # Phase 1-3 roadmap
├── ANSB.md                           # Native engine vision
├── ANS_MCP_TOOLS.md                  # MCP tool reference (nerves/)
├── install.ps1 / install.sh          # One-click installers
├── static/dashboard.html             # Dashboard UI
├── crates/                           # Rust daemon (15 crates)
│   ├── ans-cdp/                      # Chromium CDP (launch, navigate, click, etc)
│   ├── ans-ipc/                      # gRPC server + session manager
│   ├── ans-gateway/                  # MCP + REST + WebSocket
│   ├── ans-goal/                     # Goal management
│   ├── ans-signal/                   # Signal router
│   ├── ans-immune/                   # Injection defense
│   ├── ans-budget/                   # Budget tracker
│   └── ... (11 more)
├── nerves/                           # Python intelligence
│   ├── .env                          # DEEPSEEK_API_KEY + OPENAI_API_KEY
│   ├── ans_nerves/
│   │   ├── __init__.py               # .env auto-loading
│   │   ├── config.py                 # All config (DeepSeek, GPT-4o, Runtime)
│   │   ├── llm/client.py             # Prompt-only JSON, model-agnostic
│   │   ├── planner/loop.py           # AgentLoop (parallel eyes, fast mode, caching)
│   │   ├── planner/planner.py        # AgentPlanner (DOM selectors, validation)
│   │   ├── decomposer/decomposer.py  # GoalDecomposer with cache
│   │   ├── eyes/                     # 5 Eyes (dom_reader, vision, page_diff, goal_verifier, error_detector)
│   │   ├── coordinator/              # CrossEyeCoordinator
│   │   ├── scoring/                  # Decision Intelligence (intelligence, embeddings, store, scorer)
│   │   └── grpc_client.py            # gRPC client for daemon
│   ├── tests/                        # 253 tests (252 pass, 1 skip)
│   └── pyproject.toml
└── proto/                            # Protobuf definitions
```

## Key Config Values

```python
# config.py defaults:
provider: "deepseek"
model: "deepseek-v4-flash"
base_url: "https://api.deepseek.com/v1"
vision_provider: "openai"
vision_model: "gpt-4o"
embedding_model: "BAAI/bge-m3"
embedding_dim: 1024
max_tokens: 4096
temperature: 0.3
timeout: 60s
screenshot: 512x512
mode: "thorough" (override with ANS_MODE=fast)
prewarm_chrome: True
```

## Commands

```powershell
# Start daemon
.\target\release\ans-daemon.exe --grpc-port 50051 --gateway-port 50052 --nerves-dir nerves

# Run goal directly (with progress tracking)
cd nerves
.\.venv\Scripts\python.exe -m ans_nerves run "your goal"

# Decompose only (test LLM)
.\.venv\Scripts\python.exe -m ans_nerves decompose "your goal"

# Run tests
.\.venv\Scripts\python.exe -m pytest tests/ -q

# Build daemon
cargo build --release -p ans-daemon

# Install one-click
powershell -ExecutionPolicy Bypass -File install.ps1
```

## Known Limitations

1. Vision Eye hits OpenAI rate limits (30K TPM) — 512x512 helps but still tight
2. Port CLOSE_WAIT issue on Windows after daemon crash (need SO_REUSEADDR)
3. WebSocket event bridge not fully wired (broadcast channel exists, needs IpcServer → Gateway connection)
4. MCP spawn uses system python (venv auto-detection built but not in current running binary)
5. No multi-tab within a session
6. Selector matching is DOM-based now but still relies on LLM choosing from provided elements
7. BGE-M3 downloads on first use (~2GB), currently downloading

## Phase 1 Implementation Status (from ANS_ROADMAP.md)

| # | Item | Status | Notes |
|---|------|--------|-------|
| 1 | Dashboard UI | **DONE** ✓ | Live at http://127.0.0.1:50052 — dark theme, goal input, progress cards, 5 Eyes panel, WebSocket connection, mode toggle |
| 4 | Fast Mode | **PARTIAL** | RuntimeConfig added to config.py, ANS_MODE=fast env var supported. Loop.py skips Vision on clean pages (every 3rd cycle), skips PageDiff when URL unchanged. Decomposer cache done. Full fast path (skip Coordinator entirely, skip Goal Verifier mid-flow) not wired. |
| 5 | Chrome Pre-warm | **NOT DONE** | Only searches Playwright cache. No pre-launch pool. Architecture designed (BrowserPool struct in session.rs, tokio::spawn background warmer). |
| 7 | One-click Install | **DONE** ✓ | install.ps1 (Windows) and install.sh (Linux/macOS) created. Check prerequisites, build daemon, install Python deps, install Chromium, create .env, start daemon, open dashboard. |
| 9 | Streaming WebSocket | **NOT DONE** | broadcast::channel exists in ws.rs but not wired to IpcServer's GoalStateStore. Needs bridge.rs to forward goal updates + eye reports from gRPC event bus to WebSocket broadcast. Architecture designed — EventBus → bridge task → WebSocketServer::push_event() → clients. |

## Phase 2 (from ANS_ROADMAP.md)

NOT STARTED. Contains:
| # | Item |
|---|------|
| 2 | Agent profiles — Per-agent sessions, cookies, memory, API keys |
| 3 | Multi-tab — Multiple goals in parallel, switch between them |
| 6 | Memory dashboard — View/search LanceDB stored actions, analytics |
| 8 | Error recovery UI — See failures, retry, manual override |
| 10 | Config panel — Choose LLM, API keys, mode toggle |
| 11 | System tray — Background mode, desktop notifications |

## Next Session Priorities

1. Finish Fast Mode wiring in loop.py (skip Coordinator on fast mode)
2. Implement Chrome pre-warming (BrowserPool, session reuse)
3. Wire WebSocket streaming (bridge.rs — EventBus → WS clients)
4. Then benchmark tests against browser-use

## Key Decisions

- ANS is a **standalone browser**, not middleware. Chromium is the engine, ANS is the browser.
- Integration is one MCP config line. No SDK, no multi-step.
- browser-use comparison: ANS requires `create_goal()` + `check_goal()`. browser-use requires agent to handle every step manually.
- Native engine (ANSB.md) is Phase 3 — replaces Chromium with own renderer. Not started.
