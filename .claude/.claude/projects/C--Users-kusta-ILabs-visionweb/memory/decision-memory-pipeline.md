---
name: decision-memory-pipeline
description: "Full decision memory flow: score → embed → store → retrieve → validate. Cold/warm start gating, similarity matching, and how memory reduces failures/steps."
metadata:
  type: project
  originSessionId: 9a1a8898-cc98-4228-bea9-ba41a244822d
---

## Decision Memory Pipeline — Full Flow

### 1. Recording: Score → Embed → Store

After EVERY action, `DecisionIntelligence.record_action()` (`nerves/ans_nerves/scoring/intelligence.py:106`) runs:

```
Action executed (click #search-btn on search_form page)
  ↓
AdvancedMultiFactorScorer (scorer.py) — 6 dimensions:
  1. Immediate outcome (success=1.0, partial=0.4, fail=0.0)
  2. Goal advancement (sub-goal-done=1.0, advanced=0.7, criterion=0.4)
  3. Efficiency (sigmoid: 200ms click→0.85, 5000ms timeout→0.05)
  4. Consistency (past success rate, default 0.5)
  5. Business impact (long-term, default 0.0)
  6. Error penalty (NLP-classified severity, SUBTRACTED)
  
  Composite = 0.30×immediate + 0.25×goal + 0.10×efficiency 
             + 0.15×consistency + 0.10×business − 0.10×error_penalty
  ↓
Context text built: "action:click tool: selector:#search-btn goal:Find flights Delhi→Mumbai page:search_form"
  ↓
FastEmbed (bge-small-en-v1.5, 384-dim) or hash fallback
  ↓
Stored in LanceDB at <data_dir>/decisions/ table "scored_actions"
  - Fields: id, session_id, goal_id, action_type, selector, value, tool, 
    context_type, context_text, vector(384), outcome_score, result_score,
    error_penalty, business_outcome, composite_score, use_count, last_used_at
```

### 2. Retrieval: Cold Start vs Warm Start

`AgentPlanner.plan_next_action()` (`planner.py:87-93`):

| Records in LanceDB | Mode | What happens |
|---|---|---|
| 0–2 | **Cold start** | No memory lookup. LLM reasons from goal + page state + DOM elements only. `source="llm_cold"` |
| 3+ | **Warm start** | Queries LanceDB FIRST via ANN similarity, then LLM validates. |

Threshold: `_WARM_START_MIN_RECORDS = 3`

### 3. Warm Start Flow (Similarity Matching)

```
Current context → embed to 384-dim vector
  ↓
LanceDB ANN search (cosine similarity, k×2 then filter to k=5)
  ↓
Filter: composite_score ≥ _MIN_RECOMMENDATION_SCORE (0.3)
  ↓
Format recommendations for LLM prompt:
  {rank, action_type, selector, tool, composite_score, outcome_score,
   error_penalty, use_count, similarity}
  ↓
LLM prompt: "Use them as STRONG hints, but verify they make sense 
             for the CURRENT page state — override if they don't."
  ↓
┌──────────────────┬──────────────────┬─────────────────┐
│ LLM agrees       │ LLM disagrees    │ LLM call fails  │
│ source="memory_  │ source="memory_  │ fallback to top │
│ validated"       │ override"        │ recommendation  │
└──────────────────┴──────────────────┴─────────────────┘
```

**How similarity works for related tasks:**

The context text `"goal:Find flights Chennai→Delhi page:search_form"` embeds close to `"goal:Search flights Delhi→Mumbai page:search_form"` because:
- Both share `page:search_form` (strong signal)
- Goal descriptions are semantically similar (flight search)
- FastEmbed captures this even though tasks aren't identical
- Cosine similarity in LanceDB finds these neighbors

### 4. How Memory Reduces Failures

1. **Error penalty downranks bad actions** — 14 regex patterns classify errors by severity (captcha=1.0, element_not_found=0.6, timeout=0.75). Penalty subtracted from composite score, so error-prone actions rank lower.

2. **Minimum score filter** — Actions scoring < 0.3 never reach the LLM. Captcha-blocked actions drop near zero and disappear.

3. **LLM validation gate** — Prompt says "override if current page state contradicts." Memory says click `#search-btn` but page now has `#find-flights` → LLM overrides.

4. **Error gating in loop** — After 3 errors on a sub-goal (`_max_errors_per_subgoal`), escalates instead of looping forever.

### 5. How Memory Reduces Steps

Cold start: LLM reasons from scratch → may pick wrong element → error → retry → 3-5 steps to find right action.
Warm start: Query LanceDB → top candidates pre-scored → LLM validates → higher first-attempt success → 1 step.

`source` field tracks: `"memory_validated"` (memory got it right), `"memory_override"` (LLM found better), `"llm_cold"` (no memory available).

### 6. Gap: No Empirical Measurement Yet

The mechanisms exist but are NOT instrumented to prove reduction:
- No A/B metric comparing warm vs cold runs for same goals
- No error-rate-by-source tracking
- No step-count comparison across repeated goals
- `LoopResult` has `total_steps`/`total_errors` but nothing correlates them with memory mode

### Key Files

| File | Role |
|------|------|
| `nerves/ans_nerves/scoring/intelligence.py` | DecisionIntelligence — main entry point, record_action + query_best_actions |
| `nerves/ans_nerves/scoring/scorer.py` | AdvancedMultiFactorScorer — 6-dimension scoring with error classification |
| `nerves/ans_nerves/scoring/embeddings.py` | EmbeddingGenerator — FastEmbed (bge-small-en-v1.5) with hash fallback |
| `nerves/ans_nerves/scoring/store.py` | LanceDBStore — persistent vector store, ANN cosine similarity search |
| `nerves/ans_nerves/planner/planner.py` | AgentPlanner — dual-mode (cold/warm), memory validation, source classification |
| `nerves/ans_nerves/planner/loop.py` | AgentLoop — orchestrates perceive→decide→act→learn→verify cycle |
| `nerves/ans_nerves/llm/prompts.py` | PLANNER_SYSTEM prompt (line 511), build_planner_user_prompt (line 577) |

[[session-state-jun-2026]] [[phase1-complete-jun-2026]] [[decision-intelligence-layer]]
