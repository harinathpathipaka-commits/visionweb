# P5: Production Hardening — Python ANS Nerves ✅ COMPLETE

## Completion Notes (2026-05-24)

The plan understated the baseline. Phases 1-4 were already substantially built before this session. What was actually done:

- **Phase 1 gap**: Added logging (`get_logger` + `logger.warning` calls) to 5 files that silently swallowed LLM failures: `vision.py`, `page_diff.py`, `goal_verifier.py`, `error_detector.py`, `decomposer.py`. Skipped `dom_reader.py` (pure deterministic classification, no external deps).
- **Phase 3 gap**: Fixed `coordinator.py` import ordering (logger assignment was between imports).
- **Phase 4 gap**: Python CI job already existed in `.github/workflows/ci.yml` (lines 209-238) — ruff, mypy, pytest on ubuntu/macos/windows.
- **Phase 5 gap**: Added 12 LLM integration mock tests across all 4 LLM-dependent eyes (success, exception, parsed-none paths each). 207 tests pass (was 195).

## Context

P1-P4 built the full product: 5 Eyes, Coordinator, Decomposer, gRPC client, Decision Intelligence (embeddings + LanceDB + feedback loop). All 21 tests pass. But the codebase has zero production infrastructure: no structured logging, no config validation, no gRPC resilience, no CI/CD, no Docker, no CLI, and only 2 test files covering 26 source files. P5 closes these gaps.

This repo is **Python-only** (the Rust daemon lives in a separate repo — clippy warnings, Chrome matrix, and budget E2E don't apply here).

## Plan

### Phase 1: Structured Logging
**Why first**: Can't debug anything else without it.

- **New file `ans_nerves/logging.py`**: Single `configure_logging()` entry point using structlog. JSON output by default, dev/console mode with `ANS_LOG_DEV=1`. Respects `ANS_LOG_LEVEL` env var.
- **Migrate existing callers** (4 files): Replace `import logging; logging.getLogger(__name__)` with `from ans_nerves.logging import get_logger` in `grpc_client.py`, `scoring/embeddings.py`, `scoring/store.py`, `scoring/intelligence.py`.
- **Add missing log calls**: LLM fallback paths in `llm/client.py`, coordinator fallback in `coordinator/coordinator.py`, LanceDB table creation in `scoring/store.py`.
- **Init in `__init__.py`**: Call `configure_logging()` at package import.

### Phase 2: Configuration Validation (pydantic)
**Why**: Currently accepts any value silently. pydantic is already a dependency but unused.

- **Rewrite `ans_nerves/config.py`**: Replace dataclasses with pydantic `BaseModel`. `NervesConfig` uses `pydantic_settings.BaseSettings` for auto env-var loading (`ANS_` prefix). Add `Field(ge/le/gt)` validators on all numeric fields. Add model validator that scoring weights sum to ~1.0.
- **Support `ans.toml`**: Via `SettingsConfigDict(toml_file="ans.toml")`, overridable with `ANS_CONFIG_PATH`.
- **Keep backward compat**: `get_config()` and `get_api_key()` singletons unchanged.

### Phase 3: gRPC Resilience
**Why**: Zero retry, zero circuit breaker. Daemon restart kills all in-flight calls.

- **Add retry to `ans_nerves/grpc_client.py`**: Follow the tenacity pattern from `llm/client.py`. Retry on `UNAVAILABLE`, `DEADLINE_EXCEEDED`, `RESOURCE_EXHAUSTED` with exponential backoff (max 3 attempts). Skip retry on permanent errors (`INVALID_ARGUMENT`, `NOT_FOUND`).
- **New file `ans_nerves/circuit_breaker.py`**: Simple 3-state circuit breaker (CLOSED/OPEN/HALF_OPEN). Opens after 5 consecutive failures, 30s recovery timeout.
- **New file `ans_nerves/exceptions.py`**: `NervesError` base, `GrpcConnectionError`, `GrpcTimeoutError`, `CircuitBreakerOpenError`, `ConfigValidationError`.
- **Wire into `GrpcClient`**: `_call()` wrapper method with retry + circuit breaker. Refactor all 21 RPC methods to use it.

### Phase 4: CI/CD + Docker + CLI

- **`.github/workflows/ci.yml`**: Lint (ruff), type-check (mypy), test (pytest --cov) on Python 3.11/3.12/3.13, ubuntu-latest.
- **`Dockerfile`**: Multi-stage, python:3.12-slim, production deps only.
- **`ans_nerves/__main__.py`**: CLI via argparse. Subcommands: `health`, `config`, `serve`, `decompose <goal>`.
- **`ans_nerves/health.py`**: stdlib HTTP server with `/health` and `/ready` endpoints, reporting daemon connectivity and store record count.

### Phase 5: Test Coverage
**Why**: 2 test files for 26 source files. Critical paths untested.

New test files (all using `pytest` + `pytest-asyncio`, plain `unittest.mock`):

| Test file | Count | Covers |
|-----------|-------|--------|
| `tests/test_config.py` | ~8 | Validation rules, env loading, TOML loading, missing API key |
| `tests/test_embeddings.py` | ~6 | embed, fallback, empty text, batch, build_context_text |
| `tests/test_store.py` | ~8 | store, query, filter, update_outcome, count (uses tmp_path) |
| `tests/test_intelligence.py` | ~6 | record_action, query_best_actions, report_outcome, batch |
| `tests/test_eyes.py` | ~12 | Each eye: valid input, missing input, LLM failure fallback |
| `tests/test_coordinator.py` | ~6 | Empty reports, valid synthesis, LLM failure fallback |
| `tests/test_decomposer.py` | ~4 | Empty goal, valid goal, LLM failure, parsed-none fallback |
| `tests/test_grpc_client.py` | ~8 | Connect/close, retry behavior, circuit breaker, status codes |

## Execution Order

```
Phase 1 (logging) ──┐
                    ├──> Phase 3 (gRPC resilience)
Phase 2 (config)  ──┘        │
                             ├──> Phase 5 (tests)
Phase 4 (CI/Docker/CLI) ─────┘
```

Phases 1, 2, and 4 are independent and can be built in parallel. Phase 3 depends on Phase 1 (uses logging for retry warnings). Phase 5 (tests) depends on Phases 1-3 being done.

## Verification

After each phase:
- **Phase 1**: Run any test with `ANS_LOG_DEV=1`, see structured log output
- **Phase 2**: `python -c "from ans_nerves.config import NervesConfig; NervesConfig(grpc_port=99999)"` → ValidationError
- **Phase 3**: New `tests/test_grpc_client.py` tests pass
- **Phase 4**: `python -m ans_nerves config` prints JSON; CI green on first push
- **Phase 5**: `pytest -v --cov=ans_nerves` shows ≥65% coverage, all existing 21 tests still pass
