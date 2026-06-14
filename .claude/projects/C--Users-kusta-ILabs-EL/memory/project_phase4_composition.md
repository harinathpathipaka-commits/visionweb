---
name: phase4-composition-complete
description: Phase 4 complete — composition.py with 11 built-in SaaS formulas, /v2/decompose, /v2/explain, /v2/formulas, 477 tests passing
metadata:
  type: project
  originSessionId: d409ae57-c891-4778-9e11-32ea2497080d
---

Phase 4 (Composition + Decomposition) complete (2026-06-01). 1 new module, 3 new endpoints, 19 new tests.

**Why:** Phase 2 answered "which variables matter" and Phase 3 answered "what happens next." Phase 4 answers "what drives NRR?" and "why did profit change?" — the compositional reasoning enterprises need.

**How to apply:** `POST /v2/decompose` to see driver contributions. `POST /v2/explain` to attribute a change to specific drivers. `GET /v2/formulas` to see all registered formulas.

### New Module: composition.py

| Component | What |
|-----------|------|
| `FormulaRegistry` | Parse, evaluate, decompose, explain-change for named formulas |
| `BUILTIN_FORMULAS` | 11 SaaS + economics formulas (ARR, LTV, NRR, CAC payback, profit, ROI, etc.) |
| `reg.decompose(metric, values)` | Marginal contribution of each driver |
| `reg.explain_change(metric, before, after)` | Per-driver attribution of change |

### Built-in Formulas

| Formula | Expression | Example |
|---------|-----------|---------|
| arr | mrr * 12 | $100K MRR → $1.2M ARR |
| revenue | customers * arpu | 500 × $200 → $100K |
| ltv | arpu / churn_rate | $100 / 5% → $2000 |
| cac_payback_months | cac / (arpu * gross_margin) | — |
| net_revenue_retention | (start + expansion - contraction - churned) / start | 97% NRR |
| gross_profit | revenue * gross_margin | — |
| mrr_growth_rate | (new + expansion - contraction - churned) / starting | — |
| profit | revenue - cost | — |
| margin_pct | (revenue - cost) / revenue * 100 | — |
| conversion_rate | converted / total * 100 | — |
| roi | (gain - cost) / cost * 100 | — |

### New Endpoints

| Method | Path | What |
|--------|------|------|
| POST | /v2/decompose | Driver contribution breakdown: {metric, total, components} |
| POST | /v2/explain | Change attribution: {before, after, delta, drivers[]} |
| GET | /v2/formulas | List all 11 built-in formulas |

### Test Result
**477 passed, 0 failed** (458 Phase 0-3 + 19 new Phase 4)

### Next: Phase 5
Explanation engine + causal inference: src/explanation.py, src/causal.py, cohort analysis. See [[project_phase3_forecast]] for Phase 3 baseline.
