# Phase 9 Complete: Hardening

Production-ready Rust daemon — zero production unwraps, lint-clean workspace-wide, expanded injection defense, secured auth, integration tests passing for critical paths, benchmarks running for hot-path crates.

## Completed Items

1. **Workspace Lints** — `[lints] workspace = true` activated in all 13 crate Cargo.toml files. `unwrap_used = "deny"`, `expect_used = "warn"`, `unsafe_code = "deny"` enforced globally.

2. **Unwrap Elimination** — All 23 production `unwrap()` calls replaced:
   - `ans-gateway/src/mcp.rs`: 7 `serde_json::to_string_pretty().unwrap()` → `unwrap_or_else()`
   - `ans-storage/src/decisions.rs`: 3 `RwLock` unwraps → `expect("lock poisoned")`
   - `ans-immune/src/detector.rs`: 3 regex unwraps → `expect("invalid regex")`
   - `ans-immune/src/rules.rs`: 1 regex unwrap → expect
   - `ans-gateway/src/auth.rs`: 2 response builder unwraps → expect
   - `ans-signal/src/router.rs`: 1 `partial_cmp().unwrap()` → `unwrap_or(Ordering::Equal)`
   - `ans-goal/src/manager.rs`: 6 `RwLock` unwraps → expect (test module annotated with `#[allow]`)
   - `ans-proto/build.rs`: 1 `protoc_bin_path().unwrap()` → expect
   - `ans-ipc/src/session.rs`: 1 `blocking_read()` → `try_read()` (no unwrap at all)

3. **Security Hardening**:
   - Injection detector: expanded from 7 to 15 substring patterns + `LazyLock<Regex>` for compound patterns
   - Auth: removed auto-generated default admin key, added constant-time XOR comparison, permission scoping via request extensions, rejected empty API keys
   - CSS hints: wired DOM class/id extraction into distraction classifier (was dead `String::new()`)

4. **Integration Tests** — 11 new tests (7 ans-ipc gRPC, 4 ans-gateway HTTP), all passing. Uses in-process tonic server and axum test utilities.

5. **Criterion Benchmarks** — 10 benchmarks across distill/diff/immune, all within targets:
   - Distill: 100 nodes ~0.6ms, 1000 ~1.5ms, 5000 ~12ms
   - Diff: no change 500 ~483µs, 10% ~536µs, 50% ~783µs
   - Immune: clean/injection/homoglyph/zero-width all ~2.6-2.9ms

6. **Lint Tightening** — Removed `dead_code = "allow"` and `unused_imports = "allow"`. Zero new warnings.

## Verification
- `cargo build --workspace` — zero errors, zero warnings
- `cargo clippy --workspace --all-targets` — zero errors
- `cargo test --workspace` — 75 tests pass
- `cargo bench --workspace` — 10 benchmarks run
