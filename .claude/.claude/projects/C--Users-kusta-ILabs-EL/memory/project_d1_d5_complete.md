---
name: project-d1-d5-complete
description: D1-D5 adversarial/statistical/reliability/integrity/integration testing complete with all fixes applied
metadata: 
  node_type: memory
  type: project
  originSessionId: f946834a-5774-442e-94de-b9d89d454f58
---

All 5 production testing dimensions (D1-D5) executed and all 386 tests pass (2026-05-28).

**Why:** D1-D5 represented a "break the product" testing campaign covering adversarial inputs, statistical correctness, reliability under stress, data integrity, and end-to-end integration. Five production bugs were found and fixed.

**How to apply:** The full test suite (386 tests, 25 files) is the baseline for any future changes. Run `python -m pytest tests/ -q` before committing. The 5 bugs that were found inform what kind of edge cases to watch for: Pydantic float accepting NaN/Inf, csv.DictReader restval=None, numerical underflow/overflow at extreme ranges, zero-variance inputs, and module-level singleton state in the rate limiter.

**5 bugs fixed:**
1. NaN/Inf accepted as valid float input → `_reject_non_finite` model_validator
2. Zero-variance inputs crash fitting layer → pre-fitting uniqueness check
3. Ragged CSV rows cause TypeError (None[:50]) → None guard in ingestion
4. Rate limiter has no reset → `reset()` method on TokenBucket
5. Extreme-range values underflow/overflow → ptp range check before fitting

**Domain router fix:** First-match-wins replaced with weighted scoring (regex=3pts, keyword=1pt), full scoreboard logged to ledger.
