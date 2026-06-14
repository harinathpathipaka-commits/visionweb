---
name: phase5-explain-causal-complete
description: Phase 5 complete — explanation.py, causal.py, cohort.py, 6 new v2 endpoints, 495 tests passing
metadata:
  type: project
  originSessionId: d409ae57-c891-4778-9e11-32ea2497080d
---

Phase 5 (Explanation + Causal + Cohort) complete (2026-06-01). 3 new modules, 6 new endpoints, 18 new tests.

**Why:** Phase 4 broke down "what drives NRR." Phase 5 explains WHY in natural language, tests for causality, and compares across segments.

### New Modules

| Module | What |
|--------|------|
| `src/explanation.py` | explain_drivers, compare_scenarios, generate_prose — turn model results into structured NL explanations |
| `src/causal.py` | granger_causality, partial_correlation, check_confounding — temporal causality testing |
| `src/cohort.py` | split_by_category/quantile, compare_segments, compare_fits — cross-segment analysis |

### New Endpoints

| Method | Path | What |
|--------|------|------|
| POST | /v2/explain-fit | Natural language driver explanation from fit results |
| POST | /v2/causal/granger | Granger causality test (does X predict Y?) |
| POST | /v2/causal/confounding | Confounding check (does Z explain X→Y?) |
| POST | /v2/cohort/compare | Compare metric across segments |
| POST | /v2/cohort/by-category | Split by category then compare |

### What these answer

- **"Why did churn increase?"** → `/v2/explain-fit` + `/v2/causal/granger`
- **"Is the X→Y relationship real or confounded?"** → `/v2/causal/confounding`
- **"How does churn differ by segment?"** → `/v2/cohort/compare`
- **"Which cohort is most responsive to pricing?"** → `compare_fits()`

### Test Result
**495 passed, 0 failed** (477 Phase 0-4 + 18 new Phase 5)

### Growth Summary
| Phase | Tests | Modules | Endpoints |
|-------|-------|---------|-----------|
| 0 | 405 | — | — |
| 1 | 417 | ledger_query | 6 |
| 2 | 440 | multivar, categorical, discovery | 3 |
| 3 | 458 | forecast | 1 |
| 4 | 477 | composition | 3 |
| 5 | 495 | explanation, causal, cohort | 6 |
| **Total** | **495** | **9 new** | **19 new** |

### Next: Phase 6
Data pipeline integration: wire ledger solve(), smart query endpoint, close the gap between ingestion and analysis. See [[project_phase4_composition]] for Phase 4 baseline.
