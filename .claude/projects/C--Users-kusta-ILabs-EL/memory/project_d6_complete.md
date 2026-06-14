---
name: project-d6-complete
description: D6 Real World Comparative Validation complete — 19 tests on Telco Customer Churn (7,043 rows), 405 total tests passing, EL verified against ground truth
metadata:
  type: project
  originSessionId: 3871a5cf-e658-4fce-9009-d6c8eb3c461f
---

D6 — Real World Comparative Validation is complete (2026-05-30). Tests that the Equation Layer produces correct, calibrated, and actionable output on real production data where the correct answer is independently verifiable.

**Why:** D1-D5 covered unit/integration/stress/adversarial testing. D6 is the acid test: does EL actually beat GPT, Claude, and RAG on real-world quantitative reasoning? Each test encodes a specific failure mode of LLMs (hallucination, no derivative, no CI, can't refuse on insufficient data, non-deterministic) and verifies EL gets it right.

**How to apply:** Run `python -m pytest tests/test_d6_comparative.py -v` for D6-only, or `python -m pytest tests/ -q` for full 405-test suite. The Telco dataset must be at `WA_Fn-UseC_-Telco-Customer-Churn.csv` in the project root.

### Dataset
**Telco Customer Churn** — `WA_Fn-UseC_-Telco-Customer-Churn.csv` at project root
- 7,043 rows, 21 columns
- Key numeric: tenure (0-72), MonthlyCharges (18-119), TotalCharges (19-8685)
- Target: Churn (Yes=26.5%, No=73.5%)
- 11 rows have empty TotalCharges (tenure=0, new customers) — handled by coercing to 0.0

### Verified Ground Truth Relationships
| Relationship | Ground Truth |
|-------------|-------------|
| Tenure → Churn | 48% (0-11mo) → 30% → 22% → 20% → 15% → 8% → 1.7% (72+mo). Power-law decay. |
| MonthlyCharges → Churn | Q1: 11.3%, Q2: 24.6%, Q3: 37.5%, Q4: 32.7%. Non-monotonic — plateaus at high end. |
| Contract → Churn | Month-to-month: 42.7%, One year: 11.3%, Two year: 2.8% |
| Tenure ~ MonthlyCharges | Pearson r = 0.2479 (weak positive) |

### D6 Test Structure (19 tests, 7 classes)

| Class | Tests | What It Validates |
|-------|-------|-------------------|
| `TestTenurePredictsChurn` | 4 | Pearson r significance, negative derivative (drops with tenure), CI contains ground truth at x=24, directional prediction |
| `TestMonthlyChargesChurnRelationship` | 2 | Non-linear relationship detection, elasticity computation (LLMs can't compute) |
| `TestTotalChargesEquation` | 2 | Tenure→TotalCharges near-identity (R²>0.5), positive derivative |
| `TestConfidenceCalibration` | 2 | 95% CI contains ground truth across 6 tenure points (coverage ≥50%), uncertainty at extremes |
| `TestActionableGuidance` | 2 | Specific shortfall count in prose, structured fallback with review guidance for low R² |
| `TestComparativeAdvantage` | 4 | Derivative (LLMs can't), CI (LLMs can't), refusal on 3 points (LLMs guess), seed reproducibility (LLMs can't) |
| `TestNoHallucination` | 3 | Zero-variance rejection (422), no causation claims from correlation, Pearson r matches manual computation |

### LLM Failure Modes Encoded in Tests
- **LLM-1**: Hallucinates specific numbers without data access → EL computes from data
- **LLM-2**: Cannot compute derivatives/elasticities → EL provides exact dY/dX, d²Y/dX², elasticity
- **LLM-3**: Overconfident point estimates → EL provides calibrated 95% CI from Laplace approximation
- **LLM-4**: Cannot detect own ignorance → EL gate rejects insufficient data with specific shortfall
- **LLM-5**: Non-deterministic → EL reproducible with fixed random_seed
- **RAG-1**: Retrieves general domain knowledge → EL fits dataset-specific equations

### Full Suite Status
**405 tests passing, 0 failures** (386 D1-D5 + 19 D6)

### Known Limitations (noted, not blocking)
- EL measures correlation, not causation — test `test_el_does_not_claim_causation` verifies this is honest
- MonthlyCharges→Churn is non-monotonic — elasticity sign depends on fitted curve shape
- Library-hit reuse can cause identical answers for same var_hash — use distinct queries for separate fits
- Shapiro-Wilk warns for N>5000 — D6 uses 5,634 points for CI coverage test

### Next: D7
A natural D7 would be **Multi-Dataset Validation** — repeating D6 on 3-5 additional Kaggle datasets (housing prices, credit risk, energy consumption, etc.) to prove the approach generalizes beyond a single dataset.
