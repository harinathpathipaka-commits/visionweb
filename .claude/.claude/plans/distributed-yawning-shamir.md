# Phase 9: Hardening — Implementation Plan

## Context

Phases 1-8 delivered a fully functional Rust infrastructure: browser control, perception, signal routing, immune system, goal management, decision storage, and the external gateway. All 22 gRPC RPCs are live. The daemon starts both servers concurrently.

Phase 9 (from the original IMPLEMENTATION_PLAN.md roadmap) is **Hardening** — making the system production-ready. The exploration revealed specific gaps: 23 production `unwrap()` calls, 6 untested crates, workspace lints defined but not inherited, injection detector covering only 7 keywords, auth issues, and dead CSS-hint code in the distraction classifier.

**Deliverable:** "Production-ready Rust daemon — zero production unwraps, lint-clean workspace-wide, expanded injection defense, secured auth, integration tests for critical path, benchmarks for hot-path crates."

---

## Implementation Items

### 1. Activate Workspace Lints Globally

Add `[lints] workspace = true` to every crate's `Cargo.toml`. This enforces `unwrap_used = "deny"`, `expect_used = "warn"`, `unsafe_code = "deny"` across all crates. Fix all resulting warnings.

**Files:** All 12 crate `Cargo.toml` files + `ans-daemon/Cargo.toml`.

### 2. Replace Production `unwrap()` Calls (23 total)

| Location | Count | Fix |
|----------|-------|-----|
| `ans-gateway/src/mcp.rs` | 7 | Replace `serde_json::to_string_pretty().unwrap()` with `unwrap_or_else(|e| format!("serialization error: {e}"))` |
| `ans-storage/src/decisions.rs` | 3 | Replace `RwLock::read().unwrap()` / `write().unwrap()` with `?` propagation or `expect("lock poisoned")` |
| `ans-immune/src/detector.rs` | 3 | Move regex compilation to `Lazy`/`OnceLock` statics or use `expect("invalid built-in regex")` |
| `ans-immune/src/rules.rs` | 1 | Same — `Regex::new()` on constant patterns → `expect()` |
| `ans-gateway/src/auth.rs` | 2 | `Response::builder().body().unwrap()` → proper `into_response()` |
| `ans-signal/src/router.rs` | 1 | `partial_cmp().unwrap()` → `unwrap_or(Ordering::Equal)` |
| `ans-goal/src/manager.rs` | 6 | `RwLock::read().unwrap()` → expect with message |

**Approach:** For `RwLock` locks — use `expect("lock poisoned")` since lock poisoning indicates a previous panic (unrecoverable invariant). For JSON serialization — `unwrap_or_else` with fallback string. For regex — compile statically with `std::sync::LazyLock`.

### 3. Security Hardening

#### 3a. Expand Injection Detector Patterns
**File:** `crates/ans-immune/src/detector.rs`

Replace `contains_instruction_pattern()` with a comprehensive regex-based detector covering:
- "forget (all |your |previous )?instructions"
- "you are (now |a |no longer )"
- "(new |updated |replacement )?(system )?prompt"
- "STOP (everything|responding)"
- "do (not |NOT )follow"
- "the (above|previous|preceding) (text|message|content)"
- DAN/developer mode/god mode role-playing patterns
- HTML-entity encoded injection patterns

#### 3b. Fix Auth Issues
**File:** `crates/ans-gateway/src/auth.rs`

- Remove automatic default admin key generation — fail startup instead, require explicit config
- Add permission scoping: attach `KeyPermission` to request extensions so downstream handlers can check
- Add constant-time key comparison using a simple bitwise approach (no need for `subtle` crate — manual XOR accumulator)
- Reject empty-string API keys in `extract_api_key()`

#### 3c. Wire CSS Hints Into Distraction Classifier
**File:** `crates/ans-ipc/src/server.rs` (line ~495)

Replace `let css_hints = String::new()` with actual CSS class/ID extraction from the DOM node, collected during `get_distilled_dom()`.

**File:** `crates/ans-cdp/src/client.rs`

Add a helper to extract CSS class strings from DOM elements after distillation.

### 4. Integration Tests

#### 4a. `ans-ipc` Integration Tests
**New file:** `crates/ans-ipc/tests/integration_test.rs`

Test the gRPC server with a real tonic client:
- `test_health_check` — call Health RPC, verify response
- `test_create_and_close_session` — create session, verify UUID, close it
- `test_create_goal_and_check_state` — create goal, verify description/stats
- `test_submit_eye_reports` — submit reports, verify routed signal returned
- `test_store_and_query_scores` — store a decision score, query best actions
- `test_classify_distractions` — classify a known distraction pattern
- `test_scan_injections` — scan known injection payload

These tests start a real gRPC server on a random port. No Chromium needed (sessions can be created without a real browser by mocking the backend — or better, skip session operations that need CDP and focus on the RPCs that don't require a live browser).

#### 4b. `ans-gateway` Integration Tests
**New file:** `crates/ans-gateway/tests/integration_test.rs`

Test the gateway router with a real axum test server:
- `test_mcp_initialize` — send initialize request, verify protocol version
- `test_mcp_tools_list` — verify all 10 tools returned
- `test_rest_health` — GET /api/v1/health, verify 200
- `test_auth_rejected` — request without API key, verify 401

### 5. Performance Benchmarks

Add `criterion` to workspace dependencies and create benchmarks for hot-path crates.

#### 5a. DOM Distillation Benchmark
**New file:** `crates/ans-distill/benches/distill_bench.rs`

Benchmark `Distiller::process()` on:
- Small page (100 nodes) — target <1ms
- Medium page (1000 nodes) — target <10ms
- Large page (5000 nodes) — target <50ms

#### 5b. Page Diff Benchmark
**New file:** `crates/ans-diff/benches/diff_bench.rs`

Benchmark `PageDiffer::diff()` on:
- Identical pages (no change) — target <1ms
- 10% changed — target <5ms
- 50% changed — target <10ms

#### 5c. Immune System Benchmark
**New file:** `crates/ans-immune/benches/immune_bench.rs`

Benchmark combined distraction + injection scan on:
- Clean page (no distractions) — target <5ms
- Page with 5 distractions — target <10ms
- Page with injection payload — target <5ms

### 6. Tighten Lint Allowances

Remove the relaxed lint allowances from workspace `Cargo.toml`:
- `dead_code = "allow"` → remove (default warn)
- `unused_variables = "allow"` → keep for now (prototyping patterns exist)
- `unused_imports = "allow"` → remove (default warn)
- `todo = "allow"` → keep
- `print_stdout = "allow"` → keep (tracing may use println in some paths)
- `print_stderr = "allow"` → keep

Fix resulting dead code warnings by removing unused code or adding `#[allow(dead_code)]` annotations with justification comments.

---

## Files Modified / Created

| File | Action | Description |
|------|--------|-------------|
| All 13 `Cargo.toml` | Edit | Add `[lints] workspace = true` |
| `Cargo.toml` (root) | Edit | Add `criterion` dep, tighten lints |
| `crates/ans-gateway/src/mcp.rs` | Edit | Fix 7 `unwrap()` in serialization |
| `crates/ans-storage/src/decisions.rs` | Edit | Fix 3 `RwLock` unwraps |
| `crates/ans-immune/src/detector.rs` | Edit | Expand injection patterns + fix 3 regex unwraps |
| `crates/ans-immune/src/rules.rs` | Edit | Fix 1 regex unwrap |
| `crates/ans-gateway/src/auth.rs` | Edit | Remove default key gen, add permission scoping, constant-time compare |
| `crates/ans-signal/src/router.rs` | Edit | Fix 1 `partial_cmp` unwrap |
| `crates/ans-goal/src/manager.rs` | Edit | Fix 6 RwLock unwraps |
| `crates/ans-goal/src/store.rs` | Edit | Fix 9 unwraps in test code (expect) |
| `crates/ans-ipc/src/server.rs` | Edit | Wire CSS hints from DOM |
| `crates/ans-cdp/src/client.rs` | Edit | Add CSS class extraction helper |
| `crates/ans-ipc/tests/integration_test.rs` | **Create** | 7 integration tests |
| `crates/ans-gateway/tests/integration_test.rs` | **Create** | 4 integration tests |
| `crates/ans-distill/benches/distill_bench.rs` | **Create** | Distillation benchmarks |
| `crates/ans-diff/benches/diff_bench.rs` | **Create** | Diff benchmarks |
| `crates/ans-immune/benches/immune_bench.rs` | **Create** | Immune system benchmarks |

---

## What's NOT in Phase 9

- LanceDB integration (Phase 6, requires Arrow/Lance columnar storage wiring)
- Python nerves (Phase 3, separate language/ecosystem)
- TypeScript dashboard (Phase 8, separate project)
- E2E scenarios requiring LLM integration (need Python nerves first)
- Memory profiling / error injection (requires Python nerves for realistic scenarios)

---

## Verification

1. **Build:** `cargo build --workspace` — zero errors, zero warnings (except allowed categories)
2. **Clippy:** `cargo clippy --workspace --all-targets` — zero warnings
3. **Format:** `cargo fmt --all -- --check` — passes
4. **Test:** `cargo test --workspace` — all existing tests + new integration tests pass
5. **Bench:** `cargo bench --workspace` — benchmarks run, results logged
6. **Lint check:** `cargo clippy --workspace -- -D clippy::unwrap_used` — zero unwrap violations in production code
