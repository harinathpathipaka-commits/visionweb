---
name: phase2-multivar-complete
description: Phase 2 complete — multivar.py, categorical.py, discovery.py, 3 new v2 endpoints, 440 tests passing, zero new dependencies
metadata:
  type: project
  originSessionId: d409ae57-c891-4778-9e11-32ea2497080d
---

Phase 2 (Multi-variable + Categorical + Discovery) complete (2026-06-01). 3 new modules, 3 new API endpoints, 23 new tests. All pure numpy/scipy — no new dependencies.

**Why:** Phase 1 added what-if and graph capabilities. Phase 2 answers "which variables affect X?", "what drives churn?", and "how do these interact?" — the multi-variable questions enterprises actually ask.

**How to apply:** `POST /v2/solve` with Dict[str, List[float]] for multi-variable. `POST /v2/discover` for variable importance ranking. `POST /v2/correlations` for pairwise relationship discovery. Categorical variables auto-detected and encoded.

### New Modules

| Module | Purpose | Key Functions |
|--------|---------|---------------|
| `src/multivar.py` | OLS regression, prediction, importance | fit_multivariate, predict_multivariate, correlation_matrix, partial_dependence |
| `src/categorical.py` | Categorical detection and encoding | detect_categorical, auto_encode, one_hot_encode, label_encode |
| `src/discovery.py` | Variable importance ranking, pairwise discovery | rank_variables, discover_relationships |

### New API Endpoints

| Method | Path | What It Does |
|--------|------|-------------|
| POST | /v2/solve | Multi-variable OLS: given Dict[variable→values], fits y=f(x1,x2,...). Returns coefficients, std errors, p-values, variable importance, R². Optional evaluate_at for prediction. |
| POST | /v2/discover | Variable importance ranking: which predictors best explain the target. Sorted by importance with direction and p-values. |
| POST | /v2/correlations | Pairwise relationship discovery: all variable pairs above correlation threshold with significance. |

### How It Works

```
POST /v2/solve {variables: {tenure, monthly_charges, churn}, target: "churn"}
  → auto_encode(predictors)      # Detect & encode categorical vars
  → fit_multivariate(encoded, y)  # OLS with p-values, partial R²
  → predict_multivariate(...)     # Optional: evaluate at specific point
  → return coefficients + importance + p_values
```

Categorical detection heuristic: strings → categorical. Numeric with ≤2 unique → binary. Numeric with >2 unique → continuous (unless very low cardinality ratio). Binary categorical → label-encode (0/1). Multi-category → one-hot encode (drop_first).

### Test Result
**440 passed, 0 failed** (417 Phase 0-1 + 23 new Phase 2)

### Warnings (pre-existing, non-blocking)
- `multivar.py:54`: `invalid value encountered in sqrt` on perfect-fit data (singular X^T X) — handled by LinAlgError fallback.
- `fitting.py:24`: sigmoid overflow at extreme x — pre-existing, handled by sqrt fallback.

### Next: Phase 3
Time-series and forecasting: forecast.py, temporal awareness in solve(), /v2/forecast endpoint. See [[project_phase1_whatif_graph]] for Phase 1 baseline.
