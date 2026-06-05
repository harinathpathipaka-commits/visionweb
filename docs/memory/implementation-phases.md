---
name: implementation-plan
description: We build the COMPLETE product — all 4 layers including the Agent Web browser. No phasing. Everything in the architecture gets built now.
metadata: 
  node_type: memory
  type: project
  originSessionId: 13a826f6-8425-4736-961c-8836f660c858
---

## Rule

We build the **complete product** as architected — all four layers, no deferred phases.

## What gets built

| Layer | Component | Status |
|-------|-----------|--------|
| Layer 2 | Agent Web (purpose-built browser) | BUILD NOW |
| Layer 3 | Nervous System + 5 Eyes | BUILD NOW |
| Layer 4 | Decision Intelligence Layer (scoring feedback loop) | BUILD NOW |

The Agent Web is NOT a future phase. It is part of the complete product and gets built alongside everything else.

## Decision Intelligence Layer: Scoring Mechanism

After every action, the agent scores it:

```
Score(action, tool, context/task_type) = f(
    outcome,          // success or failure
    results,          // what was produced
    error_message,    // if failed — why
    business_outcome  // did the goal advance?
)
```

When facing the same context/task type again, agent picks highest-scoring (action, tool) from memory.

**Related:** [[agent-web-architecture]] [[decision-intelligence-layer]]
