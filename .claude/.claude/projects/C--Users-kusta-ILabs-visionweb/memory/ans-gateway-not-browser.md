---
name: ans-gateway-not-browser
description: "ANS is middleware that sits on top of any browser tool, not another competing browser automation tool"
metadata:
  type: project
  originSessionId: 9a1a8898-cc98-4228-bea9-ba41a244822d
---

## Rule

ANS is a **gateway/middleware**, not a browser automation tool. It does not compete with Browser-Use, Playwright, or Browserbase — it enhances whatever the user already uses.

**Why:** The value of ANS is in the intelligence layer (5 Eyes, verification, immune system, memory, decision scoring). Building another browser control layer makes ANS compete with tools users already have and trust. Instead, ANS should plug into whatever browser tool the user already uses, adding its intelligence on top.

**How to apply:** The browser control layer (ans-cdp, BrowserPool, Chrome pre-warming) must become a **pluggable backend** — one of many possible adapters. The gateway's core (eyes, verification, memory, decisions) should work with Browser-Use, Playwright, Browserbase, or any other browser automation tool via an adapter interface.

## Architecture (Revised)

```
Agent
  ↓
ANS Gateway
  ├── 5 Eyes (DOM Reader, Vision, Page Diff, Goal Verifier, Error Detector)
  ├── Immune System (distraction classification, noise suppression)
  ├── Verification (boundary verification, error gating)
  ├── Memory (LanceDB, scored actions, pattern learning)
  ├── Decision Intelligence (feedback loop, next-best-action selection)
  └── Signal Router (relevance scoring, contradiction resolution)
  ↓
Browser Adapter (pluggable)
  ├── Browser-Use adapter
  ├── Playwright adapter
  ├── Browserbase adapter
  ├── CDP-native adapter (existing ans-cdp)
  └── ...anything else
  ↓
Website
```

## Implications for Current Codebase

- `ans-cdp` becomes ONE adapter implementation, not THE browser layer
- `BrowserPool` and Chrome pre-warming belong in the CDP adapter, not in core
- WebSocket bridge, REST API, MCP server remain in gateway (they face the agent)
- All intelligence crates (ans-signal, ans-immune, ans-goal, ans-distill, ans-diff) remain core
- Need: a `BrowserAdapter` trait that all backends implement

## What ANS Competes With

- **NOT**: Browser-Use, Playwright, Browserbase, Puppeteer, Selenium
- **YES**: The intelligence gap those tools leave — perception verification, error recovery, memory, decision scoring

[[agent-web-architecture]] [[decision-intelligence-layer]] [[decision-memory-pipeline]]
