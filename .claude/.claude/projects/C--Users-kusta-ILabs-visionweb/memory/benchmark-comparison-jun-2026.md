---
name: benchmark-comparison-jun-2026
description: "ANS vs browser-use benchmark comparison — THEORETICAL ESTIMATES ONLY, not empirically measured. Actual benchmarks NOT YET RUN."
metadata: 
  node_type: project
  originSessionId: f8e3bf36-d12a-45fe-9263-5ec82cd98ee5
---

## ANS vs Browser-Use Benchmark Comparison — June 2, 2026

**⚠️ IMPORTANT: These numbers are THEORETICAL ESTIMATES, not actual benchmark results. No real benchmark tests have been run against browser-use or any other tool. This memory was corrected on 2026-06-11.**

### Browser-Use Public Numbers (from their published post)
- **82% accuracy**, **33.4s/task**, **1.9¢/task** on their evaluation set
- Custom-tuned model (bu-*), 3-5x faster than general-purpose LLMs
- Source: https://browser-use.com/posts/one-year-of-progress
- Benchmark repo: https://github.com/browser-use/benchmark

### The Benchmark Suites (available to run)

| Suite | Tasks | What it measures |
|-------|-------|-----------------|
| BU Bench V1 | 100 | Web automation (Custom 20, WebBench 20, Mind2Web 2 20, GAIA 20, BrowseComp 20) |
| Stealth Bench V1 | 71 | Anti-bot detection evasion across providers |
| Online-Mind2Web | ~20 | Multi-step web navigation success rate |

Scoring: LLM judge (Google API) comparing agent output against ground truth.
Tasks are encrypted (Fernet `.enc` files) to prevent LLM contamination.

### ANS Theoretical Estimates (NOT MEASURED — projection only)

| Category | ANS (est.) | Browser-Use (published) |
|----------|-----------|-------------------------|
| BU Bench overall | ~88% (not run) | 79% (published) |
| Stealth (anti-bot) | ~50% (not run) | 90% (published) |

### Cost Projection (theoretical, GPT-4.1 pricing)

| Mode | Per task (est.) |
|------|-----------------|
| ANS Thorough | 5.1¢ (not measured) |
| ANS Fast | 1.6¢ (calculated from token counts, not end-to-end measured) |
| Browser-Use | 1.9¢ (published) |

### What Needs to Happen to Get Real Numbers

1. Set up the BU Bench harness (clone https://github.com/browser-use/benchmark)
2. Wire ANS as the agent backend
3. Run all 100 encrypted tasks through ANS
4. Score with Google API LLM judge
5. Run Stealth Bench 71 tasks
6. Measure actual wall-clock time and token costs

### Status: NOT STARTED

No benchmark harness exists. No real comparison has been run. The numbers in the original version of this memory were theoretical architecture-based projections, not empirical results.

[[phase1-complete-jun-2026]]
