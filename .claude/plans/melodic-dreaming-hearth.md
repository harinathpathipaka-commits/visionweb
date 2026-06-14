# LLM Outcome Classifier — Hybrid Classification for MCP Gateway

## Context

The MCP gateway currently relies on agent-provided `task_type` to classify outcomes. When the agent doesn't pass it (or passes garbage), everything pools into the `"unknown"` bucket, making the scoring engine blind. The user wants a hybrid approach: trust explicit agent fields when present, use GPT-4o mini to extract task_type, context, error_message, result_summary, and business_outcome from raw tool calls when agent data is missing or noisy.

## Architecture Decision

**Where it runs**: Inside `logDecision()` in `gateway-proxy.ts`. This is already fire-and-forget (`void`) — no latency added to the agent's tool call response.

**Merge strategy**: Agent-provided > LLM-inferred > rule-based defaults. If the agent explicitly passes `task_type: "payment_failed"`, the LLM is skipped entirely for that field.

**Fail-open**: If the LLM times out (3s) or errors, fall back to existing rule-based defaults. The system never blocks on LLM availability.

**LLM Provider**: OpenAI-compatible API (GPT-4o mini), following the `schema-inferrer.ts` `LLMProvider` pattern. Supports any OpenAI-compatible endpoint.

**Caching**: LRU cache keyed on `(tool_name, task_signature_hash)` for task_type classification. Avoids redundant LLM calls for repeated tool invocations on the same task pattern.

## Files

### NEW: `packages/mcp-server/src/outcome-classifier.ts`

The LLM classifier module. Structure:
- `OutcomeClassification` interface — the structured output from the LLM
- `ClassifierConfig` — timeout, model, base URL, api key, cache TTL
- `OutcomeClassifier` class:
  - `classify(toolName, args, result, isError, agentTaskType)` → `OutcomeClassification | null`
  - Internal: builds prompt from tool call context, calls LLM, parses JSON response
  - Internal: LRU cache for task_type lookups
  - Internal: result truncation (max 2000 chars to control token cost)

### MODIFY: `packages/mcp-server/src/types.ts`

Add `OutcomeClassification` type:
```typescript
export interface OutcomeClassification {
  task_type: string;
  context: string;
  success: boolean;
  error_message: string | null;
  result_summary: string;
  business_outcome: 'resolved' | 'partial' | 'failed' | 'unknown';
  confidence: number;
}
```

### MODIFY: `packages/mcp-server/src/config.ts`

Add to `GatewayConfig`:
```typescript
classifier: {
  enabled: boolean;
  apiKey: string | null;     // falls back to OPENAI_API_KEY
  model: string;              // default 'gpt-4o-mini'
  baseUrl: string;            // default 'https://api.openai.com/v1'
  timeoutMs: number;          // default 3000
  cacheTtlMs: number;         // default 300000 (5 min)
} | null;
```

New env vars (all optional, classifier disabled by default):
- `LI_CLASSIFIER_ENABLED` — `"true"` to enable
- `LI_CLASSIFIER_API_KEY` — falls back to `OPENAI_API_KEY`
- `LI_CLASSIFIER_MODEL` — default `"gpt-4o-mini"`
- `LI_CLASSIFIER_BASE_URL` — default `"https://api.openai.com/v1"`
- `LI_CLASSIFIER_TIMEOUT_MS` — default `3000`
- `LI_CLASSIFIER_CACHE_TTL_MS` — default `300000`

### MODIFY: `packages/mcp-server/src/gateway-proxy.ts`

1. Extend `logDecision()` record to include `args?: Record<string, unknown>`, `result?: UpstreamCallResult`, and `isError?: boolean`
2. In `handleCallTool()`, pass `args` and `result` to `logDecision()`
3. In `logDecision()`:
   - After building base payload, if classifier is enabled:
     - Check if agent provided a valid task_type (not "unknown"/empty)
     - If missing → call `classifier.classify(toolName, args, result, isError, agentTaskType)`
     - Merge: `agent_explicit > llm_inferred > defaults`
     - Build enriched payload with: task_type, context, error_message (from LLM), business_outcome
   - If classifier disabled or unavailable → existing behavior (unchanged)

### MODIFY: `packages/mcp-server/src/index.ts`

Wire up classifier creation during server startup:
- Read classifier config from `loadConfig()`
- If enabled, create `OutcomeClassifier` instance
- Pass to `GatewayProxy` constructor

## LLM Prompt Design

System prompt (terse, structured):
```
You are an outcome classifier. Given a tool call and its result, extract structured fields.
Classify the task_type from this taxonomy: payment_failed, refund_processing, subscription_management, ticket_resolution, auth_recovery, order_recovery, onboarding, code_deployment, data_query, content_generation, web_search, file_operation, system_diagnostic, unspecified_issue.
Determine business_outcome: resolved (success), partial (mixed/partial success), failed (error/unsuccessful), unknown (unclear).
Output ONLY valid JSON matching the schema. No explanation.
```

User prompt (variable, built per call):
```json
{
  "tool_name": "<tool>",
  "arguments": {"key": "value", ...},
  "result": "<first 2000 chars of result content>",
  "is_error": false,
  "agent_task_type": "unknown"
}
```

Expected LLM output (parsed + validated):
```json
{
  "task_type": "code_deployment",
  "context": "Pushing a bug fix commit to main branch",
  "success": true,
  "error_message": null,
  "result_summary": "Commit pushed successfully, PR #1234 created",
  "business_outcome": "resolved",
  "confidence": 0.95
}
```

## Verification

1. `npm run build` in `packages/mcp-server` — 0 TypeScript errors
2. `npm test` in `packages/mcp-server` — all 28 existing tests pass
3. Manual test: set `LI_CLASSIFIER_ENABLED=true`, `OPENAI_API_KEY=sk-...`, start server, make a tool call without `task_type` → verify outcome logged with LLM-inferred task_type
4. Manual test: make a tool call with explicit `task_type` → verify LLM is skipped, explicit value used
5. Manual test: set `LI_CLASSIFIER_ENABLED=false` (or omit) → verify existing behavior unchanged
6. Manual test: kill LLM (wrong API key) → verify graceful fallback, outcome still logged with defaults
