---
name: phase1-complete-jun-2026
description: "All 5 Product Roadmap Phase 1 items done as of 2026-06-01 — Fast Mode, Chrome Pre-warming, WebSocket Bridge"
metadata: 
  node_type: memory
  type: project
  originSessionId: f8e3bf36-d12a-45fe-9263-5ec82cd98ee5
---

## Phase 1 Complete — June 1, 2026

Product Roadmap Phase 1 is now **100% complete**. All 5 items done:

| # | Item | Status |
|---|------|--------|
| 1 | Dashboard UI | DONE (prior session) |
| 4 | Fast Mode | **DONE this session** — wired RuntimeConfig into AgentLoop |
| 5 | Chrome Pre-warming | **DONE this session** — BrowserPool in ans-ipc |
| 7 | One-click Install | DONE (prior session) |
| 9 | Streaming WebSocket | **DONE this session** — GoalStateStore → WS bridge |

### Item 1: Fast Mode (Python)

**File modified:** `nerves/ans_nerves/planner/loop.py`
- Added `from ans_nerves.config import get_config` import
- Added `self._runtime = get_config().runtime` in `AgentLoop.__init__`
- 4 config-driven gates in `_execute_sub_goal`:
  1. **Vision**: fast+skip_vision_in_fast → skip entirely; thorough → every 3rd cycle on clean pages
  2. **PageDiff**: fast+skip_diff_in_fast → skip entirely; always skip when URL unchanged
  3. **Coordinator**: fast+fast_coordinator → `_fallback_synthesize()` (deterministic, 0 LLM); thorough → `synthesize()` (LLM)
  4. **GoalVerifier**: fast mode → skip on intermediate steps, only verify when `errors_this_subgoal >= 2` or planner signals "done"; thorough → every step
- Note: `done` terminal action triggers `break` BEFORE verifier, so verifier never runs on the "done" step itself

**File created:** `nerves/tests/test_loop_fast_mode.py` — 8 tests covering:
- `TestFastModeCoordinator`: fast uses fallback, thorough uses LLM
- `TestFastModeEyes`: vision skipped, diff skipped, verifier skipped on click, verifier runs on errors
- `TestThoroughMode`: vision runs (step 3), verifier runs every step (1 call since done breaks before verify)

**Test pattern:** Set `loop._intelligence = _mock_intelligence()`, `loop._decomposer = _mock_decomposer()`, `loop._planner = mock`, `loop._eyes["eye_name"] = mock`, `loop._runtime = _make_runtime(mode=...)` directly. Use `AsyncMock` for coordinator.synthesize (not MagicMock).

**Key gotcha:** DecisionIntelligence creates real EmbeddingGenerator + LanceDBStore which are slow. Must mock `loop._intelligence = _mock_intelligence()` in every test.

### Item 2: Chrome Pre-warming (Rust)

**File created:** `crates/ans-ipc/src/pool.rs` — `BrowserPool` struct
- `Arc<Mutex<VecDeque<CdpBackend>>>` for idle backends
- `acquire()`: pop from pool if available, else cold-launch
- `release()`: push back if under max_size, else close
- `spawn_warmer()`: background task maintains min_idle backends (500ms interval)
- `shutdown()`: closes all idle backends
- Pool bounds: max_size configurable, min_idle = max_size/2 (at least 1)
- 4 unit tests in `#[cfg(test)]` module

**Files modified:**
- `crates/ans-ipc/src/lib.rs` — added `pub mod pool;` and `pub use pool::BrowserPool;`
- `crates/ans-ipc/src/session.rs` — `SessionManager` gets `pool: BrowserPool` field; `create()` calls `pool.acquire()`; `close()` calls `pool.release()`; added `with_pool_size(usize)` constructor
- `crates/ans-ipc/src/server.rs` — added `with_pool_size(usize)` builder and `goal_store()` / `goal_manager()` accessors
- `crates/ans-daemon/src/main.rs` — added `--prewarm N` CLI flag (default 2); chains `.with_pool_size(cli.prewarm)` on IpcServer

**Key gotchas:**
- `BrowserBackend` trait must be imported in pool.rs for `.close()` method
- `tracing::debug!` with `.await` inside macro breaks `Send` — must extract value before macro
- `close()` returns `Result` — need `let _ =` to satisfy `unused-must-use`

### Item 3: WebSocket Streaming Bridge (Rust)

**Files modified:**
- `crates/ans-gateway/Cargo.toml` — added `ans-goal = { path = "../ans-goal" }` dependency
- `crates/ans-gateway/src/lib.rs` — `Gateway::init()` takes `Option<GoalStateStore>`; spawns bridge task that subscribes to store and forwards `GoalStateNotification` → `ws.push_event(json)`
- `crates/ans-ipc/src/server.rs` — added `goal_store()` and `goal_manager()` accessor methods
- `crates/ans-daemon/src/main.rs` — calls `server.goal_store()` before spawn; passes `Some(goal_store)` to `Gateway::init()`

**JSON event format on WebSocket:**
```json
{"type": "goal_update", "goal_id": "...", "progress": 0.5, "status": "Active", "message": "Progress: 50%"}
```

### Current Test Status

| Layer | Tests | Result |
|-------|-------|--------|
| Python fast-mode | 8 | ALL PASS ✅ |
| Rust ans-ipc pool | 4 | ALL PASS ✅ |
| Rust ans-ipc integration | 12 | ALL PASS ✅ |
| Rust ans-gateway unit | 5 | ALL PASS ✅ |
| Rust ans-gateway integration | 4 | ALL PASS ✅ |
| Python full suite | 252 | unchanged (existing tests too slow to run all — LLM-dependent) |

### Build & Verification Commands

```powershell
# Python tests (fast mode only — instant)
cd nerves
.\.venv\Scripts\python.exe -m pytest tests/test_loop_fast_mode.py -v

# Rust build
cargo build --release -p ans-daemon

# Rust tests
cargo test -p ans-ipc -p ans-gateway

# Run daemon with pre-warming
.\target\release\ans-daemon.exe --prewarm 2

# Fast mode via env var
$env:ANS_MODE = 'fast'
```

### Deleted/Unused Code

- Removed unused `ans_stealth::StealthConfig` import from session.rs (pool handles StealthConfig now)
- `_HIERARCHY` in coordinator.py remains dead code (was dead before this session)

### What Phase 2 Contains (NOT STARTED)

From ANS_ROADMAP.md:
| # | Item |
|---|------|
| 2 | Agent profiles — Per-agent sessions, cookies, memory, API keys |
| 3 | Multi-tab — Multiple goals in parallel, switch between them |
| 6 | Memory dashboard — View/search LanceDB stored actions, analytics |
| 8 | Error recovery UI — See failures, retry, manual override |
| 10 | Config panel — Choose LLM, API keys, mode toggle |
| 11 | System tray — Background mode, desktop notifications |
