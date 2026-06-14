---
name: phase1-whatif-graph-complete
description: Phase 1 complete — /v2/what-if endpoint, EquationGraph wired into pipeline, ledger_query.py, graph path/variables endpoints, 417 tests passing
metadata:
  type: project
  originSessionId: d409ae57-c891-4778-9e11-32ea2497080d
---

Phase 1 (What-if + Equation Graph + Ledger Queries) complete (2026-06-01). 3 new API endpoints, 1 new module, graph integration wired into solve pipeline.

**Why:** Phase 0 fixed critical bugs. Phase 1 adds the first capabilities that directly address the enterprise SaaS "real questions" — counterfactual evaluation, multi-hop derivation, and data retrieval from the observation ledger.

**How to apply:** `POST /v2/what-if` to evaluate stored equations at arbitrary x. `POST /v2/graph/path` for multi-hop derivation paths. `GET /v2/variables` for ledger data. Graph auto-registers equations from solve() calls.

### What Was Built

| Item | Type | What |
|------|------|------|
| `/v2/what-if` | Endpoint | Evaluate stored equation at arbitrary x-value. No re-fitting — reads from library. Returns answer + derivative + elasticity + CI. |
| `/v2/graph/path` | Endpoint | Find derivation path from source to target variable through registered equations. Returns edges with equation metadata. |
| `/v2/graph/variables` | Endpoint | List all variables in the shared equation graph. |
| `/v2/graph/summary` | Endpoint | Graph stats: node count, edge count, variable list. |
| `/v2/variables` | Endpoint | List variables in observation ledger with observation counts. |
| `/v2/variables/{name}` | Endpoint | Time-series data for a specific ledger variable. |
| `src/ledger_query.py` | Module | Observation retrieval: query_observations, query_time_series, query_pair (aligned), get_latest_value, list_variables_in_ledger. |
| Graph wiring | Pipeline | EquationGraph initialized from DB library on-demand. New equations auto-registered after fitting. Library hits also registered. |

### Architecture

```
solve() → _fit_and_store() → _register_in_graph()     [new fit]
solve() → _equation_from_store() → _register_in_graph() [library hit]
             ↓
     get_equation_graph(db_path) →  _load_graph_from_library()
             ↓
     /v2/graph/*  ←  get_equation_graph()  ←  shared EquationGraph
```

Module-level `_equation_graph` in pipeline.py shared across all API calls.

### New Endpoints Summary

| Method | Path | Auth Required |
|--------|------|---------------|
| POST | /v2/what-if | Yes |
| POST | /v2/graph/path | Yes |
| GET | /v2/graph/variables | Yes |
| GET | /v2/graph/summary | Yes |
| GET | /v2/variables | Yes |
| GET | /v2/variables/{name} | Yes |

### Test Result
**417 passed, 0 failed** (405 existing + 12 new Phase 1 tests)

### Next: Phase 2
Multi-variable support: multivar.py, categorical.py, discovery.py, multi-variable SolveV2Request. See [[project_phase0_bugs_fixed]] for Phase 0 baseline.
