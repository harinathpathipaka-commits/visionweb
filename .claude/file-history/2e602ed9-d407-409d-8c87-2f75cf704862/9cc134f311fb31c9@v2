# Plan: Production-Grade D7 Forecast & Statistical Validation

## Context

D7 tests (32 passing) target the Store Sales Kaggle dataset but are shallow:
- **Forecast**: No train/test split, no accuracy metrics, no baseline comparison — just sanity checks
- **Statistical**: No holdout validation, no residual diagnostics, no VIF, no F-test, no CV
- 3 tests are `assert True` no-ops

The user wants production-grade validation using EL's actual pipeline on real data.

## What Gets Changed

### File: `tests/test_d7_store_sales.py` — rewrite with production-grade tests

Keep good tests (CSV ingest, encoding, driver ranking, API endpoints) as-is. Replace the weak forecast and statistical tests.

### Forecast Validation (5 NEW tests, replaces `TestD7_Forecast`)

| Test | What it validates | Assertion |
|------|-------------------|-----------|
| `test_train_test_mape_vs_naive` | EL beats naive mean forecast | Train on first 90% of daily series, forecast last 10%. MAPE < naive MAPE |
| `test_accuracy_vs_persistence` | EL beats last-value persistence | RMSE < persistence RMSE |
| `test_ci_coverage_rate` | 95% CI actually captures future | ≥65% of actuals fall in CI (allows for real-world noise) |
| `test_residuals_no_autocorr` | Residuals are white noise | Durbin-Watson between 1.5–2.5 |
| `test_direction_accuracy_above_chance` | Forecast direction is predictive | Sign of change correct >45% of steps |

### Statistical Correctness (8 NEW tests)

| Test | What it validates | Assertion |
|------|-------------------|-----------|
| `test_holdout_r2_stable` | Fit generalizes | 80/20 split. Holdout R² ≥ train R² × 0.5 |
| `test_f_test_overall_significance` | Model explains variance | F-test p < 0.05 |
| `test_residual_normality_jarque_bera` | Residuals approx normal | Skewness ∈ [-1,1], Kurtosis ∈ [2,5] |
| `test_vif_below_threshold` | No multicollinearity | All VIF < 10 |
| `test_promotion_positive_coefficient_when_present` | Domain knowledge holds | If promotions present, coefficient > 0 |
| `test_durbin_watson_no_autocorr` | Residuals not autocorrelated | DW ∈ [1.5, 2.5] |
| `test_cross_validation_r2_positive` | 5-fold mean R² > 0 | All folds have positive R² |
| `test_ci_exponential_widening` | Uncertainty grows with horizon | CI width at h=15 > CI width at h=1 |

### What we keep from existing:
- All `TestD7_CSVIngest` (5 tests)
- All `TestD7_Encoding` (3 tests)  
- All `TestD7_DriverRanking` (4 tests — good, uses partial R²)
- `TestD7_FitQuality::test_r2_is_meaningful`, `test_n_obs_matches` (keep basic checks)
- `TestD7_WhatIf` — keep but soften (promotions are very sparse)
- `TestD7_API` (3 tests — good API integration tests)
- `TestD7_Ranges` (4 tests — keep sanity checks)

### Key implementation details:
- Use `scope="module"` fixtures to share expensive CSV reads (122MB file)
- Fit OLS once for statistical tests via shared fixture
- For CV: use sklearn `KFold` but fall back to manual split if sklearn unavailable
- All statistical computations use scipy (already a dependency)
- VIF: `VIF_j = 1 / (1 - R²_j)` where R²_j is from regressing predictor j on all other predictors
- Durbin-Watson: `Σ(e_t - e_{t-1})² / Σ(e_t)²`
- Jarque-Bera: `n/6 * (S² + (K-3)²/4)`

## Verification

```bash
python -m pytest tests/test_d7_store_sales.py -v --tb=short
```

Expected: ~40-45 tests, all passing. Runtime ~5-7 minutes (dominated by CSV read).
