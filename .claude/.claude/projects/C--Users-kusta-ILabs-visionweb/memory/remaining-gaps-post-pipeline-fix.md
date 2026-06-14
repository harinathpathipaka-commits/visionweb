---
name: remaining-gaps-post-pipeline-fix
description: "Gaps remaining after June 13 pipeline fix session — Agent Web browser, Cross-Eye Awareness, Immune System, Signal Router, and P2 items deferred for post-benchmark"
metadata:
  type: project
  originSessionId: 013c3991-068b-422d-a793-bba962f25c2a
---

## Remaining Gaps — Post Pipeline Fix (June 13, 2026)

All 22 P0/P1 pipeline bugs were fixed in this session (proto ↔ Rust ↔ Python connections, visible_text, SOM, verifier, error detector, re-decomposition, stagnation detection). These are the gaps that remain — deferred until after benchmark runs.

---

## Gap 1: Custom Agent Web Browser (ARCHITECTURAL — deferred post-benchmark)

**Original design** (`agent-web-architecture.md`): A purpose-built browser for AI agents, built from scratch. Renders structured perception, not visual pixels. DOM distilled at render time, goal relevance scored before agent sees anything, distractions blocked at intake, sessions organized by goal (not tabs), multiple simultaneous views.

**Current state**: Using Chrome via CDP. Works functionally but:
- Chrome was built for humans (tabs, bookmarks, visual viewport)
- DOM distillation happens AFTER render, not during
- Distractions reach the agent before being classified
- No parallel goal views, no built-in goal state as a browser primitive
- Page diff is a separate RPC call, not automatic on every load

**Decision**: Build the custom Agent Web browser AFTER benchmark validation. Current Chrome CDP approach is sufficient to prove the concept works.

---

## Gap 2: Cross-Eye Awareness (P2 — deferred)

**Original design**: Eyes share information laterally. Vision Eye sees an overlay → DOM Reader knows to mark those elements as blocked. DOM Reader finds a form → Vision Eye focuses on form fields. Page Diff detects a new element → all eyes re-evaluate.

**Current state**: Eyes run independently in parallel via `asyncio.gather()`. The coordinator merges their outputs AFTER they all complete. No lateral sharing DURING observation.

**Impact**: Vision might report "element #3 is visible" while DOM Reader simultaneously reports "element #3 has `is_visible=false`" — the contradiction is caught by the coordinator but could be avoided entirely with real-time sharing.

**Fix needed**: Eyes should receive each other's preliminary outputs before finalizing their own. Or the coordinator should run iteratively, feeding partial results back to eyes.

---

## Gap 3: Immune System — Active Blocking (P2 — deferred)

**Original design** (`agent-web-architecture.md`): "Distractions classified and suppressed at intake." The immune system should BLOCK popups, cookie banners, ads, and modals BEFORE the agent sees them — not just flag them.

**Current state**: The Rust distiller classifies distractions into `DistractionFlag` (kind, selector, confidence, suggested_action). These flags are in the DOM output but nothing ACTS on them. The planner might see a cookie banner as just another interactive element.

**Fix needed**: After distillation, automatically execute `suggested_action` (dismiss/block/suppress) on high-confidence distraction flags before the agent's perception step begins.

---

## Gap 4: Signal Router — Underutilized (P2 — deferred)

**Original design**: "Scores relevance, suppresses noise, amplifies goal signals." The Signal Router has a 5-level contradiction resolution hierarchy and relevance scoring. It should produce a scored, ranked, noise-filtered perception — not just a text merge.

**Current state**: The coordinator's `_fallback_synthesize()` does simple text concatenation. `synthesize()` (LLM) is now called for contradictions, but the Signal Router's full scoring pipeline (relevance weights, noise suppression, signal amplification) is not integrated into the coordinator output.

**Fix needed**: Wire the Signal Router's relevance scores into the coordinator output. Elements should be ranked by goal relevance, not just listed. Noise (ads, nav bars, footers) should be demoted or hidden.

---

## Gap 5: Continuous Decision Intelligence (P2 — deferred)

**Original design** (`decision-intelligence-layer.md`): "Inbuilt feedback loop, not middleware. The agent sees its history of actions → outcomes → adjusts its next move. Scores every action per context/task type."

**Current state**: Warm-start works (3+ records → LanceDB query → LLM validation). But:
- Only action-level scoring, not task-level patterns ("on layerinfinite.app, always avoid the Google sign-in button")
- `business_outcome` field exists but is rarely populated
- No cross-task pattern learning — each goal starts fresh
- Rust DecisionStore is populated via gRPC sync but never queried independently

**Fix needed**: Task-level pattern storage ("site X needs strategy Y"), cross-goal memory sharing, populate business_outcome from verifier outcomes more consistently.

---

## Gap 6: Coordinator LLM Selectivity (P2 — deferred)

**Current state**: The LLM `synthesize()` is called when contradictions exist or diff is meaningful. Otherwise the deterministic fallback runs. This is good but the conditions could be smarter — the LLM coordinator should run when:
- Multiple eyes disagree (confidence gap > 0.3 between any two eyes)
- Page type changed unexpectedly
- Vision confidence is low (< 0.4)

**Fix needed**: Add confidence-gap detection and page-type-change detection to the coordinator routing logic.

---

## Gap 7: Python Test Coverage (P2 — deferred)

**Current state**: 252+ tests exist but many are for the Rust side. Python loop, eyes, and coordinator have minimal test coverage.

**Fix needed**: Add integration tests for:
- Full perception → action → verify cycle with mocked gRPC
- SOM annotation output validation
- Re-decomposition triggering
- Coordinator contradiction resolution
- Verifier screenshot path

---

## Gap 8: Benchmark Harness (NEXT PRIORITY)

**Current state**: No real benchmarks. All numbers are theoretical estimates. BU Bench (browser-use's 100-task benchmark) is the target.

**Fix needed**: Build a benchmark harness that runs real tasks against ANS and browser-use side by side, measuring completion rate, steps per task, cost per task, and time per task.

---

## Priority Order (post-benchmark)

1. Benchmark harness + real runs against browser-use
2. Custom Agent Web browser (if benchmark shows CDP is the bottleneck)
3. Immune System active blocking
4. Cross-Eye Awareness
5. Signal Router integration
6. Continuous Decision Intelligence
7. Test coverage

[[agent-web-architecture]] [[decision-intelligence-layer]] [[pipeline-bugs-verified-jun-2026]] [[session-state-jun-2026]] [[benchmark-comparison-jun-2026]]
