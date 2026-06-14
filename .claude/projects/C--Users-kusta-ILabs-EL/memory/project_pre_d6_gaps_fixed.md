---
name: pre-d6-gaps-fixed
description: 5 API response gaps fixed before D6 comparative validation — derivative, elasticity, fallback_action, correlation, drift now surfaced
metadata:
  type: project
  originSessionId: 3871a5cf-e658-4fce-9009-d6c8eb3c461f
---

5 critical API response gaps were identified and fixed (2026-05-30) before D6 Real World Comparative Validation could run.

**Why:** The API computed derivative, elasticity, correlation, drift, and fallback actions internally but never serialized them in the response. D6 comparative testing requires these fields to demonstrate EL's advantage over LLMs (which cannot compute any of them).

**How to apply:** The new fields are backward-compatible (all Optional with None default). API consumers should check for `derivative`, `curvature`, `elasticity`, `pearson_r`, `drift_detected`, and `fallback_action` in SolveResponse. All existing tests pass unchanged.

### 5 Gaps Fixed

| # | Gap | Root Cause | Fix |
|---|-----|-----------|-----|
| 1 | `derivative`/`elasticity` returned "N/A" | `ComputeResult` had them, `SolveResponse` didn't serialize | Added to `SolveResult` + `SolveResponse`; compute elasticity via `sensitivity_map()` in pipeline |
| 2 | `fallback_action` always `null` | `route_failure()` ran but result was scoped inside if-block, never stored in `SolveResult` | Moved `fallback = None` before if-block; stored `asdict(fallback)` in `SolveResult`; added to `SolveResponse` |
| 3 | Gate inconsistent across runs | Hardcoded seed 42 in `split_holdout()`, no user control | Added `random_seed: Optional[int]` to `SolveRequest`, `solve()`, `split_holdout()`, `fit_canonical_families()` |
| 4 | Correlation/drift computed but invisible | Local variables in `solve()`, not stored in `SolveResult` | Added `pearson_r`, `drift_detected` fields to `SolveResult`; populated from existing local variables |
| 5 | Fallback prose generic | `_format_template()` ignored `FallbackAction` structured data | Rewrote fallback prose to use `title`, `description`, and `suggested_next_steps` from `FallbackAction`; updated `_build_context()` for LLM path |

### Files Changed (6 files, ~40 lines net)

| File | Change |
|------|--------|
| `src/models.py` | Added `fallback_action`, `elasticity`, `pearson_r`, `drift_detected` to `SolveResult` (all with defaults) |
| `src/api.py` | Added `random_seed` to `SolveRequest`; added `derivative`, `curvature`, `elasticity`, `pearson_r`, `drift_detected`, `fallback_action` to `SolveResponse`; wired in endpoint |
| `src/fitting.py` | Threaded `random_seed` through `split_holdout()` → `fit_canonical_families()` (default=None preserves hardcoded 42 behavior) |
| `src/pipeline.py` | Added `sensitivity_map()` call for elasticity; captured fallback outside if-block; populated new `SolveResult` fields; threaded `random_seed` |
| `src/formatter.py` | Rich fallback prose from `FallbackAction` dict; updated `_build_context()` with fallback data |
| `tests/test_d5_integration.py` | Updated `SOLVE_FIELDS` to include 6 new response keys |

### Key Design Decisions
- `fallback_action` stored as `Dict[str, Any]` (not `FallbackAction`) — avoids circular import (fallback_router.py imports from models.py)
- derivative/curvature/elasticity flattened to scalars (not nested dicts) for API simplicity
- `sensitivity_map()` wrapped in try/except — can fail on degenerate equations
- `random_seed` default=None preserves existing behavior (hardcoded seed 42)

### Test Impact
All 386 D1-D5 tests pass with zero modifications (except SOLVE_FIELDS constant update). New fields are additive and optional.
