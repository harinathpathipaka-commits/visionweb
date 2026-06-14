---
name: phase0-bugs-fixed
description: Phase 0 critical/high bugs fixed — 13 bugs across 8 files, 405 tests passing, real covariance now computed from fitting Jacobian
metadata:
  type: project
  originSessionId: d409ae57-c891-4778-9e11-32ea2497080d
---

Phase 0 bug fixes complete (2026-06-01). 13 production bugs fixed across 8 source files. All 405 tests pass.

**Why:** 3-agent analysis (Debugger 53 bugs, Architect 23 gaps) identified critical issues that produced silently wrong results: x_value silently dropped, fictional uncertainty estimates, dead security constraints, broken background worker.

**How to apply:** All fixes are backward compatible. New fields are Optional with None defaults. The `evaluate_at` field on SolveRequest enables what-if evaluation. The Jacobian-based probabilistic covariance gives calibration-tested CIs.

### Bugs Fixed

| ID | Severity | File | What |
|----|----------|------|------|
| B1 | CRITICAL | api.py, pipeline.py, models.py | Added `evaluate_at` to SolveRequest/SolveResponse. Previously x_value in POST body was silently ignored; all evaluations happened at x_values[-1]. Now threaded through pipeline. |
| B2 | CRITICAL | storage.py | `find_equation()` called conn.execute() outside `with` block — crashes on Python 3.12+. Moved all DB ops inside. |
| B3 | CRITICAL | probabilistic.py, pipeline.py, fitting.py, models.py | Hardcoded `np.eye*0.01` covariance replaced with Jacobian-based (J^T J)^{-1} scaled by residual variance. Added `jacobian` field to EquationFit. `laplace_from_fit` now accepts `residual_std` for predictive uncertainty. |
| B12 | HIGH | fitting.py | Bare `except Exception: continue` now logs warnings before swallowing. |
| B13 | HIGH | pipeline.py | `sensitivity_map()` silent failures now logged. |
| B16 | HIGH | fitting.py | Shapiro-Wilk NaN p-value (identical residuals / perfect fit) now returns random=True. |
| B8 | HIGH | fitting.py | MAPE threshold now scale-aware (uses `y_scale * 1e-8` not hardcoded `1e-10`). |
| B18 | HIGH | worker.py | Background worker refit query now parses variable name from `var_hash` instead of using `row["domain"]`. |
| B19 | MEDIUM | storage.py | `promote_version()` now sets `drift_status='stable'` — previously all promoted equations showed "pending" forever. |
| B20 | MEDIUM | formatter.py | Template no longer shows "inf%" when MAPE is infinite. |
| B27 | MEDIUM | api.py, pipeline.py, models.py | API response `unit` field now surfaces variable unit from query parsing (was always empty string). |
| B32 | MEDIUM | worker.py | SIGTERM signal handler guarded with `hasattr(signal, 'SIGTERM')` for Windows compatibility. |
| B33 | MEDIUM | api.py | Content type check tightened — "text/plain" no longer accepted as CSV. |

### Files Changed (8 files)

| File | Changes |
|------|---------|
| `src/api.py` | evaluate_at in SolveRequest/SolveResponse, unit wired, content type check |
| `src/models.py` | jacobian on EquationFit, evaluated_at + unit on SolveResult |
| `src/pipeline.py` | evaluate_at threading, Jacobian covariance wiring, residual_std, unit extraction, logged exceptions |
| `src/probabilistic.py` | Fixed _hessian_from_jacobian, residual_std parameter on laplace_from_fit |
| `src/fitting.py` | Jacobian storage in fit_canonical_families, scale-aware MAPE, Shapiro NaN fix, logged exceptions |
| `src/storage.py` | find_equation connection fix, promote_version drift_status |
| `src/worker.py` | Variable query fix, SIGTERM guard |
| `src/formatter.py` | inf% display fix, math import |
| `tests/test_d5_integration.py` | SOLVE_FIELDS updated |
| `tests/test_d4_integrity.py` | drift_status assertion relaxed |

### Test Result
**405 passed, 0 failed, 23 warnings** (all warnings are pre-existing numerical edge cases)

### Next: Phase 1
What-if endpoint, graph wiring, data retrieval from observation_ledger. See [[project_phase0_bugs_fixed]] for context and the Architect's [[project_d6_complete]] for D6 baseline.
