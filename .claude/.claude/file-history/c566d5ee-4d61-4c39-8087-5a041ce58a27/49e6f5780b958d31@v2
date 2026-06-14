# Production-Grade Outcome Capture — Implementation Plan

## Context

6 gaps identified in the outcome capture pipeline that prevent production-readiness:
1. Queue processor (`processQueue()`) has no caller — queued outcomes rot on disk
2. `DecisionTracker` fully implemented but never instantiated/wired
3. Layer 2/3 webhook callbacks require manual external integration — no auto-registration of pending signals
4. No end-to-end tests for the full outcome pipeline
5. No rule-based fallback when GPT-4o mini is unavailable (single point of failure)
6. LLM `business_outcome` at low confidence triggers false-positive silent failure alerts

## Approach

### Gap 1: Queue processor has no caller
**File**: `src/fail-open.ts`
- Add `startQueueProcessor(send, intervalMs)` function that runs `processQueue()` immediately on startup, then on a 30s interval
- Returns `{ stop: () => void }` for clean shutdown
- Wire in `index.ts` → `createGatewayServer()`

### Gap 2: DecisionTracker unwired
**File**: `src/index.ts`, `src/gateway-proxy.ts`, `src/decision-tracker.ts`
- Instantiate `DecisionTracker` in `createGatewayServer()` with the `LiApiClient`
- Add to `GatewayServer` return type
- Pass to `GatewayProxy` constructor
- In `GatewayProxy.logDecision()`: call `tracker.recordDecision()` for episode tracking + secondary disk durability (`.li-queue/decisions.jsonl`)
- Start `tracker.startFlushTimer()` on startup, stop on shutdown
- The tracker's buffer flush acts as a **secondary** durability path — the primary is `logDecision()` → `liApi.logOutcome()` → `enqueueOutcome()` (individual JSON files)

### Gap 3: Layer 2/3 auto-registration
**File**: `src/gateway-proxy.ts`
- When LLM classifier produces `business_outcome: 'partial' | 'unknown'` OR confidence < 0.7, set `feedback_signal: 'delayed'` in the payload
- This leverages the existing API behavior: `feedback_signal: 'delayed'` auto-creates a `signal_pending: true` row in `fact_outcomes` with `cross_event_status: 'pending_signal'`
- No new API endpoints needed — uses existing `dim_pending_signal_registrations` flow
- When external webhook arrives with Layer 2/3 data, the existing `handleWebhookCallback()` resolves the pending signal

### Gap 4: End-to-end tests
**File**: `__tests__/outcome-pipeline.test.ts` (new)
- Test 1: Full pipeline — `handleCallTool` → `logDecision` → verifies payload structure (all required fields present)
- Test 2: Classifier skip logic — when agent provides explicit task_type AND call succeeds, LLM is skipped
- Test 3: Merge precedence — agent-explicit > LLM-inferred > defaults
- Test 4: Rule-based fallback — when LLM unavailable, `classifyWithRules()` produces valid classification
- Test 5: Queue fallback — when `liApi.logOutcome()` fails, payload is enqueued to disk
- Test 6: Confidence gating — `business_outcome` only sent when confidence >= 0.7
- Test 7: `feedback_signal: 'delayed'` set when confidence < 0.7
- Test 8: DecisionTracker episode lifecycle — start → record → flush

### Gap 5: Rule-based fallback classifier
**File**: `src/outcome-classifier.ts`
- Add exported `classifyWithRules(toolName, args, result, isError, agentTaskType)` function
- Heuristics:
  - `result.isError === true` → `business_outcome: 'failed'`, `success: false`
  - Result text contains error keywords (`Error`, `failed`, `denied`, `refused`, `invalid`, `timeout`) → `business_outcome: 'failed'`, `success: false`
  - Result text is empty/null → `business_outcome: 'unknown'`, `success: true`
  - Otherwise → `business_outcome: 'resolved'`, `success: true`
  - Always sets `confidence: 0.5` (neutral — rule-based is uncertain by nature)
  - `task_type` kept from `agentTaskType` if valid, else `'unspecified_issue'`
- Called from `GatewayProxy.logDecision()` when `classifier.classify()` returns `null`

### Gap 6: Silent failure false positives
**File**: `src/gateway-proxy.ts`
- Gate `business_outcome` on confidence: only attach LLM's `business_outcome` to payload when `classification.confidence >= 0.7`
- When confidence < 0.7: omit `business_outcome` from payload (DB gets `null`) AND set `feedback_signal: 'delayed'`
- This prevents low-confidence LLM classifications from triggering `detectSilentFailure()` alerts
- Also sets `classification_confidence` in payload so the API/dashboard can show confidence level

## Files Modified

| File | Change |
|------|--------|
| `src/fail-open.ts` | Add `startQueueProcessor()` |
| `src/outcome-classifier.ts` | Add `classifyWithRules()` rule-based fallback |
| `src/gateway-proxy.ts` | Wire DecisionTracker, confidence gating, rule-based fallback, delayed signal logic |
| `src/index.ts` | Instantiate DecisionTracker, start queue processor + flush timer, wire shutdown |
| `__tests__/outcome-pipeline.test.ts` | New — 8 end-to-end tests |

## Verification

```bash
cd packages/mcp-server
npm run build          # Must compile with zero errors
npm test               # All existing + new tests must pass
```

Verify specific behaviors:
1. `npm test` — 8 new pipeline tests pass alongside existing 28 tests
2. TypeScript compilation catches any wiring mistakes (DecisionTracker added to GatewayProxy constructor)
3. Queue processor starts on gateway creation, stops on shutdown (no leaks)
