---
name: agent-web-architecture
description: "The Agent Web is a purpose-built browser for agents — renders for agent perception, not human eyes. Information from the real web flows in and is processed before the agent sees it."
metadata: 
  node_type: memory
  type: project
  originSessionId: 13a826f6-8425-4736-961c-8836f660c858
---

## Rule

The Agent Web is a **purpose-built browser for AI agents**, built from scratch — not Chrome with extensions bolted on. It renders web content for agent perception, not human eyes.

**Why:** Chrome was built for humans (tabs, bookmarks, history, one viewport, visual pixels). Agents need parallel goal tracking, structured goal state, action verification logs, perception eyes built-in, session context per goal, multiple simultaneous views, and goal extraction — none of which Chrome provides natively. Bolting these onto Chrome is the wrong foundation.

**How to apply:** The Agent Web is the intake and processing layer that sits between the real internet and the Nervous System. Every capability that current systems bolt onto Chrome externally (Playwright, vision model calls, DOM distillation, page diff, goal state) is native infrastructure in the Agent Web.

## The Four-Layer Architecture (Complete)

```
Layer 1: THE REAL WEB (websites unchanged, as they are)
              │
              │ information flows in
              ▼
Layer 2: AGENT WEB (purpose-built browser for agents)
  ├── Intake Layer: distraction classification on arrival, DOM distillation at render time,
  │                 goal relevance scored before agent sees it, vision pipeline auto-triggered
  ├── Session Layer: sessions organized by goal (not tabs), parallel sessions share goal state,
  │                  page diff runs automatically on every load
  └── Infrastructure: browser control, vision pipeline, distillation, diff, goal state — ALL native
              │
              ▼
Layer 3: NERVOUS SYSTEM + EYES (goal-directed perception)
  ├── 5 Eyes: DOM Reader, Vision Model, Page Diff, Goal Verifier, Error Detector
  ├── Signal Router: scores relevance, suppresses noise, amplifies goal signals
  ├── Immune System: structurally stronger than any distraction
  └── Cross-Eye Awareness: lateral information sharing between all eyes
              │
              ▼
Layer 4: DECISION INTELLIGENCE LAYER (inbuilt feedback loop, NOT middleware)
  ├── Action history + outcomes → next best action/tool selection
  ├── Error pattern learning → avoids repeating failures
  └── Token-efficient: failure patterns in working context prevent retries
```

## Key Distinction: Agent Web vs Chrome

| Chrome (human browser) | Agent Web (agent browser) |
|---|---|
| Renders visual pixels | Renders structured perception layers |
| Tabs = unrelated pages | Sessions = goal-linked parallel views |
| Extensions bolted on | Perception eyes built-in |
| One viewport | Multiple simultaneous views |
| Popups shown then dismissed | Popups classified and suppressed at intake |
| Goal state external | Goal state is a browser primitive |
| Page diff a separate tool | Diff runs automatically on every load |

## Information Flow

Real Web → Intake (distraction flagged, DOM distilled, relevance scored) → Session (goal-organized, parallel) → Nervous System (eyes perceive, signals routed) → Decision Intelligence (feedback loop selects next action) → Agent acts on a clean, goal-relevant, distraction-free structured view

## Status

This is the most important architecture. The Agent Web is the foundation everything else rests on. [[decision-intelligence-layer]]
