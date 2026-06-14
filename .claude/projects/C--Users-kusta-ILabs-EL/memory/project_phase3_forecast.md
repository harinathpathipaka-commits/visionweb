---
name: phase3-forecast-complete
description: Phase 3 complete — forecast.py with trend/seasonality/decomposition/forecast, /v2/forecast endpoint, 458 tests passing
metadata:
  type: project
  originSessionId: d409ae57-c891-4778-9e11-32ea2497080d
---

Phase 3 (Time-Series Forecasting) complete (2026-06-01). 1 new module, 1 new endpoint, 18 new tests. Pure numpy/scipy.

**Why:** Phase 2 answered "which variables matter." Phase 3 answers "what happens next quarter?" — the temporal forecasting questions enterprises need.

**How to apply:** `POST /v2/forecast` with time-series values and horizon. Auto-detects seasonality. Returns forecasts with expanding confidence intervals.

### New Module: forecast.py

| Function | What |
|----------|------|
| `extract_trend(y, x)` | Linear trend: slope, intercept, R² |
| `detect_seasonality(y)` | Autocorrelation-based period detection |
| `decompose(y, period)` | Additive decomposition: trend + seasonal + residual |
| `forecast(y, horizon, period)` | Trend extrapolation + seasonal pattern + expanding CIs |
| `forecast_from_ledger(db, var, horizon)` | Convenience: query ledger then forecast |

### New Endpoint

| Method | Path | What |
|--------|------|------|
| POST | /v2/forecast | Time-series forecast: values + horizon → forecasts + CI bands |

### How It Works

```
POST /v2/forecast {values: [100...300], horizon: 4}
  → detect_seasonality(values)       # Autocorrelation → period=12 (monthly)
  → extract_trend(values)            # Linear trend slope + intercept
  → decompose(values, period)        # Trend + seasonal + residual
  → forecast(horizon)                # trend_extrapolation + seasonal_repeat
  → CI widening: σ * sqrt(1 + h)     # Uncertainty grows with horizon
  → return forecasts + ci_lower + ci_upper
```

### Test Result
**458 passed, 0 failed** (440 Phase 0-2 + 18 new Phase 3)

### Next: Phase 4
Composition and driver decomposition: formula registry in ontologies, composition engine, /v2/decompose endpoint, /v2/explain. See [[project_phase2_multivar]] for Phase 2 baseline.
