# Fix 4 remaining clippy errors for production readiness

## Context

`cargo clippy --workspace --all-targets` reports 5 errors (3 unique) blocking the build from being fully clean. These are the last blockers to production-grade status.

## Errors and fixes

### 1. Gateway panic on auth failure (`crates/ans-gateway/src/lib.rs:110`)

**Error:** `unwrap()` used on a `Result` value (workspace lint: `unwrap_used = "deny"`)

**Fix:** Replace `.unwrap()` with `.expect("building a 401 response is infallible")`

The response builder with status + string body is truly infallible. `.expect()` documents why and satisfies the lint (workspace allows `expect_used` at warn level). This appears in both lib and lib-test targets (same code), so one fix resolves both errors.

### 2. Missing `DistillMode` import (`crates/ans-bench/benches/concurrent.rs`)

**Error:** `cannot find type DistillMode in this scope` — uses `DistillMode::AllFields` on line 72 but never imports the type.

**Fix:** Add `use ans_core::distill::DistillMode;` to the imports (after line 11). `ans-core` is already in `ans-bench/Cargo.toml` dependencies.

### 3. Missing `std::fmt::Write` import (`crates/ans-bench/benches/throughput.rs`)

**Error:** `cannot write into std::string::String` — uses `write!(html, ...)` macro on line 168 without importing the `Write` trait.

**Fix:** Add `use std::fmt::Write;` to the imports.

## Verification

```powershell
cargo clippy --workspace --all-targets  # should show 0 errors
cargo build --workspace --lib           # should succeed
cargo test --workspace                  # should pass
```
