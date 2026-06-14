# Fix 3 EL Multivariate Pipeline Bugs

## Context

During D6 Telco Churn validation, 3 bugs were found in EL's multivariate pipeline:
1. OLS on binary targets produces negative probabilities (churn = -20.5%)
2. One-hot collinearity gives identical importance to redundant columns
3. Small intercepts display as 0.0000 due to aggressive rounding

All fixes use numpy/scipy only — zero new dependencies.

---

## Bug 1: Add Logistic Regression for Binary Targets

**File:** `src/multivar.py`

### Changes

1. Add `_is_binary_target(y)` — returns True iff exactly 2 unique values both ≈ {0, 1}
2. Add `fit_logistic(X, y, add_intercept=True)` — IRLS logistic regression:
   - Initialize beta = zeros
   - Iterate: p = sigmoid(Xβ), W = p(1-p), z = Xβ + (y-p)/W, beta = WLS solution
   - Clamp p to [1e-12, 1-1e-12] to prevent division by zero
   - Max 100 iterations, tol 1e-6 on ||beta change||
   - Returns same dict shape as `fit_multivariate` + `model_type: "logistic"`
   - McFadden pseudo-R², Wald-based importance
3. Add 3-line gate at top of `fit_multivariate()`:
   ```python
   if _is_binary_target(y):
       return fit_logistic(X, y, add_intercept=add_intercept)
   ```
4. Add `model_type: "ols"` to existing OLS return for consistency

---

## Bug 2: Grouped Partial R² for One-Hot Columns

**File:** `src/discovery.py` (`rank_variables`)

### Changes

Replace lines 36-46 (per-column max-aggregation) with grouped partial R²:

1. Build groups: `{original_name: [encoded_col_indices]}` using `startswith(f"{name}_")`
2. Build full design matrix from encoded columns
3. For each group, drop ALL columns in that group simultaneously, refit OLS
4. `group_importance = R²_full - R²_reduced` (clamped ≥ 0)
5. Single-column groups (numeric passthrough) use existing per-column importance from `fit_multivariate`
6. Normalize all group importances to sum to 1

---

## Bug 3: Fix Intercept Rounding

**File:** `src/csv_analyzer.py` line 163

### Change
```python
# Before:
"coefficients": {k: round(v, 4) for k, v in ...}
# After:  
"coefficients": {k: round(v, 6) for k, v in ...}
```

---

## Files Modified
- `src/multivar.py` — Bug 1 (new functions + routing gate)
- `src/discovery.py` — Bug 2 (grouped partial R²)
- `src/csv_analyzer.py` — Bug 3 (1-line rounding fix)

## Verification
1. Run `python scripts/d6_telco_test.py` — churn predictions now in [0,1], no negative probabilities, intercept not 0.0000, no duplicate importance scores for collinear one-hot columns
2. Run `python -m pytest tests/ -q` — all 522+ existing tests must pass (continuous targets route to OLS unchanged)
