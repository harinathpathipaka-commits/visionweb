---
name: phase6-smart-query-complete
description: Phase 6 complete — smart_query.py, /v2/query capstone endpoint, full NL→variable→fit→explain pipeline, 505 tests
metadata:
  type: project
  originSessionId: d409ae57-c891-4778-9e11-32ea2497080d
---

Phase 6 (Data Pipeline Integration) complete (2026-06-01). The capstone — ties ingestion → storage → analysis → explanation into one endpoint.

**Why:** Phase 5 gave us all the building blocks. Phase 6 wires them together so a user can ask a natural language question and get a complete answer with data, fit, and explanation — all auto-detected from the observation ledger.

### New Module: smart_query.py

`run_smart_query(db_path, question)` orchestrates the full pipeline:

```
NL question → domain routing → variable extraction → ledger lookup → solve → explain
```

Key functions:
- `resolve_variables(db_path, question)` — Parse NL question, match to ledger variables, retrieve data
- `_match_variables()` — Fuzzy variable matching against ledger + SaaS pair heuristics
- `run_smart_query()` — Full pipeline: resolve → fit → explain → related drivers

### New Endpoint

| Method | Path | What |
|--------|------|------|
| POST | /v2/query | Ask a NL question → auto-detect variables → retrieve from ledger → fit → explain |

### Example

```json
POST /v2/query {"question": "how does churn change with tenure"}
→ {
  "question": "...",
  "domain": "finance.saas.smb",
  "x_variable": "tenure",
  "y_variable": "churn",
  "n_observations": 7043,
  "fit": {family, r2_adj, pearson_r, answer, ci_95, derivative, elasticity},
  "answer": "'churn' has a power relationship with 'tenure' (R²=0.480)...",
  "related_drivers": [{variable, importance, direction}]
}
```

This is how you answer "why did churn increase?" end-to-end.

### Test Result
**505 passed, 0 failed** (495 Phase 0-5 + 10 new Phase 6)

### Complete Journey

| Phase | Tests | What |
|-------|-------|------|
| 0 | 405 | 14 bugs fixed |
| 1 | 417 | What-if, graph, ledger queries |
| 2 | 440 | Multi-variable, categorical, discovery |
| 3 | 458 | Time-series forecast |
| 4 | 477 | Composition, decomposition |
| 5 | 495 | Explanation, causal, cohort |
| 6 | **505** | **Smart query — NL question → complete answer** |

10 new modules, 20 new v2 endpoints, 100 new tests from 405 to 505.
