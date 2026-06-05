# ANS MCP Tools Reference

## Integration

```json
{
  "mcpServers": {
    "ans": {
      "url": "http://127.0.0.1:50054/mcp"
    }
  }
}
```

No API keys, no SDK, no config. Just a URL. Keys are in `nerves/.env`.

---

## Tool Categories

### Intelligence Tools (USE THESE)

These trigger the full ANS pipeline — Decomposer → Planner → 5 Eyes → Coordinator → Decision Intelligence. The agent delegates and ANS handles everything.

| # | Tool | Params | What it does | When to use |
|---|------|--------|-------------|-------------|
| 1 | `create_goal` | `description` (required), `context` (optional) | Creates a goal. Daemon spawns Python AgentLoop: decomposes into sub-goals, plans actions, opens Chrome, executes every step, runs 5 Eyes verification, coordinates results, scores and learns. Returns `goal_id`. | **Always.** This is the primary tool. Every task starts here. |
| 2 | `check_goal` | `goal_id` (required) | Returns live progress: sub-goals completed/total, current step, alerts, contradictions, confidence score. | After `create_goal`, poll this until done. |

**Recommended agent workflow:**

```
1. create_goal(description="Book cheapest flight Delhi to Mumbai June 5")
   → { "goal_id": "abc-123", "status": "active" }

2. check_goal(goal_id="abc-123")
   → { "sub_goals_completed": 3, "total_sub_goals": 6, "status": "in_progress" }

3. check_goal(goal_id="abc-123")
   → { "sub_goals_completed": 6, "total_sub_goals": 6, "status": "completed",
       "summary": "Booked IndiGo 6E-213 at $94. All criteria verified." }
```

---

### Browser Control Tools (USE ONLY IF NEEDED)

These give the agent raw browser control. They bypass the intelligence pipeline — no decomposition, no verification, no recovery. The agent is driving manually.

| # | Tool | Params | What it does |
|---|------|--------|-------------|
| 3 | `create_session` | `goal_id` (required), `start_url` (optional) | Opens a new Chrome tab. Returns `session_id`. |
| 4 | `navigate` | `session_id`, `url` (required) | Navigates browser to URL. |
| 5 | `click` | `session_id`, `selector` (required) | Clicks element matching CSS selector. |
| 6 | `type_text` | `session_id`, `selector`, `value` (required) | Types text into input element. |
| 7 | `scroll` | `session_id` (required), `value` (optional) | Scrolls page. Value: "down", "up", or pixel amount. |
| 8 | `screenshot` | `session_id` (required), `full_page` (optional) | Captures screenshot as base64 PNG. |
| 9 | `get_dom` | `session_id` (required), `mode` (optional) | Gets distilled DOM. Modes: `text_only` (readable text), `input_fields` (forms), `all_fields` (debug). |
| 10 | `execute_action` | `session_id`, `action_type` (required), `selector`, `value` (optional) | Generic action runner. Types: `click`, `type`, `scroll`, `select`, `navigate`, `submit`, `wait`, `screenshot`. |

---

## Decision Matrix: Which Tools Should the Agent Use?

### For autonomous tasks → Intelligence tools only

```
create_goal + check_goal
```

The agent states the goal. ANS decomposes, plans, executes, verifies, recovers, and learns. The agent just polls for results. This is how ANS is designed to work.

### For supervised tasks → Intelligence + Browser

```
create_goal → check_goal → (if stuck) navigate/click/type_text → check_goal
```

ANS drives. If ANS reports an issue (e.g., "captcha detected"), the agent uses browser tools to help, then calls `check_goal` to let ANS resume.

### For manual browser control → Browser tools only

```
create_session → navigate → get_dom → click → type_text → screenshot
```

The agent does everything manually. No ANS intelligence. Equivalent to raw Playwright/Puppeteer. Only use this if you specifically need manual control.

---

## Rules for Agent Integration

1. **Always start with `create_goal`.** Never skip it.
2. **Poll `check_goal`.** Don't assume success — ANS reports verified progress.
3. **Trust ANS over browser tools.** If ANS says "criteria met: false," don't navigate away. Let ANS recover.
4. **Browser tools are escape hatches.** Use only when ANS reports a blocking issue it can't handle (captcha, login wall, bot detection).
5. **One goal per session.** Don't reuse sessions across goals. `create_goal` handles session lifecycle internally.

---

## What Happens Inside `create_goal`

```
create_goal("Find cheapest laptop on Amazon under $500")
    │
    ├─ Decomposer (DeepSeek V4-Flash)
    │   └─ Goal → 6 sub-goals with measurable success criteria
    │      "Sub-goal 1: Navigate to amazon.com"
    │      "Sub-goal 2: Search for 'laptops under $500'"
    │      "Sub-goal 3: Sort by price low to high"
    │      ...
    │
    ├─ Planner
    │   └─ Sub-goals → Action sequences
    │      click("#search"), type_text("#search", "laptops"), ...
    │
    ├─ Browser (Chrome via CDP)
    │   └─ Executes each action on the real website
    │
    ├─ 5 Eyes (verify every action)
    │   ├─ DOM Reader: checks element states
    │   ├─ Vision (GPT-4o): screenshot analysis
    │   ├─ Page Diff: what changed since last action
    │   ├─ Goal Verifier: did this advance the goal
    │   └─ Error Detector: classifies failures, proposes recovery
    │
    ├─ Cross-Eye Coordinator
    │   └─ Resolves contradictions between eyes
    │      "Vision says popup visible, DOM Reader says no popup"
    │      → DOM wins (per hierarchy: DOM > Vision for element detection)
    │
    └─ Decision Intelligence
        └─ Scores every action → embeds → stores in LanceDB
           Future similar tasks retrieve best-scored actions
```

---

## Ports

| Port | Service | Protocol |
|------|---------|----------|
| 50051 | gRPC daemon | tonic (internal, Python ↔ Rust) |
| 50054 | MCP + REST + WebSocket | axum (external, Agent ↔ ANS) |

Start daemon: `.\target\release\ans-daemon.exe --grpc-port 50051 --gateway-port 50054 --nerves-dir nerves`
