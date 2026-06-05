---
name: decision-intelligence-layer
description: "The Decision Intelligence Layer is an inbuilt feedback loop, not middleware — it executes/recommends the next best action based on prior actions and outcomes"
metadata: 
  node_type: memory
  type: project
  originSessionId: 13a826f6-8425-4736-961c-8836f660c858
---

## Rule

The Decision Intelligence Layer is a **feedback loop** built directly into the agent's core execution cycle — not middleware, not an external interceptor.

**Why:** Middleware adds latency, creates separation between decision and execution, and can't shape the agent's own reasoning. An inbuilt loop means the agent itself learns from every action outcome (results, errors, tool choices) and uses that accumulated context to recommend or execute the next best action/tool.

**How to apply:** When designing the execution loop, bake the feedback mechanism into the agent's own decision path. The agent sees its history of actions → outcomes → adjusts its next move. This is a first-class feature of the agent architecture, not a plugin.

## Scoring Mechanism

After every action execution, the agent scores it:

```
Score(action, tool, context/task_type) = f(
    outcome,          // success or failure
    results,          // what was produced
    error_message,    // if failed — why it failed
    business_outcome  // did the goal actually advance?
)
```

When facing a similar context/task type again, the agent picks the highest-scoring (action, tool) from memory. Simple scoring table, not complex ML. The agent learns from its own execution history.

## What it does

- Scores every (action, tool) per context/task type based on outcomes, results, errors, business outcome
- Selects next best action/tool by looking at scored memory for that context
- Prevents: wrong actions, wrong tool selection, unnecessary token usage, repeating failed patterns

## Status

This is the defining architectural constraint for the execution engine. Everything else serves this loop.
