This is a fundamental architectural question. Let me trace through every component systematically.

  ---
  The core difference

  ANS today is Agent → ANS → Chromium → Web. Removing Chromium means ANS itself becomes the rendering surface. The question is: what does an agent browser
  actually need to render?

  An agent doesn't need pixel-perfect layout, smooth animations, WebGL, or CSS transitions. It needs: structured content, interactive element positions,
  form state, and page semantics. That dramatically reduces scope versus a human browser.

  ---
  What STAYS (zero changes)

  These layers are above the rendering engine — they don't care what renders the page:

  ┌───────────────────────────┬─────────────────────────┬─────────────────────────────────────────────────────────────────────────────────────────────┐
  │         Component         │      Crate/Module       │                                        Why it stays                                         │
  ├───────────────────────────┼─────────────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────┤
  │ Gateway                   │ ans-gateway             │ MCP (10 tools), REST (8 endpoints), WebSocket, auth, metrics. Same API surface, same ports. │
  ├───────────────────────────┼─────────────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────┤
  │ gRPC contract             │ ans-proto               │ All 28 RPCs, 7 services. Same protobuf definitions.                                         │
  ├───────────────────────────┼─────────────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────┤
  │ Signal Router             │ ans-signal              │ 7-stage pipeline, 5-level contradiction resolution. Same logic, different input source.     │
  ├───────────────────────────┼─────────────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────┤
  │ Budget Enforcer           │ ans-budget              │ 4-mode circuit breaker. Unchanged.                                                          │
  ├───────────────────────────┼─────────────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────┤
  │ Goal Management           │ ans-goal                │ Goal decomposition, verification, tracking. Same semantics.                                 │
  ├───────────────────────────┼─────────────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────┤
  │ Decision Intelligence     │ ans_nerves/scoring/     │ Multi-factor scorer, LanceDB with 1536-dim embeddings, feedback loop. All unchanged.        │
  ├───────────────────────────┼─────────────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────┤
  │ Cross-Eye Coordinator     │ ans_nerves/coordinator/ │ Synthesizes 5 EyeReports → RoutedSignal. Same pipeline.                                     │
  ├───────────────────────────┼─────────────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────┤
  │ Goal Decomposer           │ ans_nerves/decomposer/  │ Goal → SubGoal DAG via GPT-4o-mini. Unchanged.                                              │
  ├───────────────────────────┼─────────────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────┤
  │ Agent Planner             │ ans_nerves/planner/     │ Cold/warm start, LanceDB query, agent loop. Unchanged.                                      │
  ├───────────────────────────┼─────────────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────┤
  │ LLM Client                │ ans_nerves/llm/         │ GPT-4o-mini via AsyncOpenAI, tenacity retry. Unchanged.                                     │
  ├───────────────────────────┼─────────────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────┤
  │ CI/CD, Docker, Benchmarks │ Various                 │ Same pipelines, same gates.                                                                 │
  └───────────────────────────┴─────────────────────────┴─────────────────────────────────────────────────────────────────────────────────────────────┘

  ---
  What CHANGES (substantial rework)

  ┌─────────────────────┬──────────────────────────────────────┬────────────────────────────────────────────────────────────────────────────────────────┐
  │      Component      │               Current                │                                        Becomes                                         │
  ├─────────────────────┼──────────────────────────────────────┼────────────────────────────────────────────────────────────────────────────────────────┤
  │ ans-cdp             │ Chromium launch, CDP WebSocket, 18   │ Deleted. Replaced by native renderer control.                                          │
  │                     │ command builders                     │                                                                                        │
  ├─────────────────────┼──────────────────────────────────────┼────────────────────────────────────────────────────────────────────────────────────────┤
  │                     │ Walks Chromium DOM via               │ Walks native DOM directly. Same 3 modes, same 9 block types, but the DOM is ours — no  │
  │ ans-distill         │ DOM.getDocument CDP call             │ CDP round-trip, no serialization cost. Distillation is now a DOM tree walk, not a      │
  │                     │                                      │ network call.                                                                          │
  ├─────────────────────┼──────────────────────────────────────┼────────────────────────────────────────────────────────────────────────────────────────┤
  │ ans-diff            │ Compares before/after DOM snapshots  │ Compares native DOM states directly. Same 7 change classifications, same               │
  │                     │ from CDP                             │ element-identity matching. Faster (no serialization).                                  │
  ├─────────────────────┼──────────────────────────────────────┼────────────────────────────────────────────────────────────────────────────────────────┤
  │ ans-ipc Session     │ Owns Chromium process handles, CDP   │ Owns rendering context handles. Each session = an isolated rendering sandbox with its  │
  │ Manager             │ WebSocket connections                │ own DOM, cookies, viewport. Same Arc<RwLock<>> pattern.                                │
  ├─────────────────────┼──────────────────────────────────────┼────────────────────────────────────────────────────────────────────────────────────────┤
  │ Screenshot          │ CDP Page.captureScreenshot           │ Native layout → rasterization → PNG. Simpler for agents: no anti-aliasing needed, no   │
  │                     │                                      │ subpixel rendering. Just element bounding boxes + text.                                │
  ├─────────────────────┼──────────────────────────────────────┼────────────────────────────────────────────────────────────────────────────────────────┤
  │ Click/Type/Scroll   │ CDP Input.dispatchMouseEvent,        │ Direct DOM event synthesis. Click = find element at (x,y) in layout tree → trigger     │
  │                     │ Input.dispatchKeyEvent               │ click handler chain. Type = focus element → insert text → fire input event.            │
  ├─────────────────────┼──────────────────────────────────────┼────────────────────────────────────────────────────────────────────────────────────────┤
  │ Navigate            │ CDP Page.navigate                    │ Native HTTP fetch → HTML parse → DOM build → CSS compute → layout. Same external       │
  │                     │                                      │ behavior, entirely native pipeline.                                                    │
  ├─────────────────────┼──────────────────────────────────────┼────────────────────────────────────────────────────────────────────────────────────────┤
  │ 5 Eyes              │ 4 of 5 call GPT-4o-mini with         │ Same LLM calls, but data comes from native DOM instead of CDP. Vision Eye gets native  │
  │ implementations     │ CDP-derived data                     │ raster output instead of CDP screenshot. DOM Reader walks native DOM directly —        │
  │                     │                                      │ becomes deterministic and fast.                                                        │
  ├─────────────────────┼──────────────────────────────────────┼────────────────────────────────────────────────────────────────────────────────────────┤
  │ Immune System       │ Scans HTML strings from CDP          │ Scans HTML at parse time — can block injection before it enters the DOM. Stronger.     │
  ├─────────────────────┼──────────────────────────────────────┼────────────────────────────────────────────────────────────────────────────────────────┤
  │ ans-stealth         │ Anti-detection + humanized CDP       │ Deleted. No browser to detect. Being a native renderer IS the stealth — there's no     │
  │                     │ interaction patterns                 │ navigator.webdriver, no CDP runtime flag, no automation fingerprint.                   │
  └─────────────────────┴──────────────────────────────────────┴────────────────────────────────────────────────────────────────────────────────────────┘

  ---
  What is NEW (build from scratch)

  This is the actual browser engine. Ordered by dependency:

  1. HTML Parser

  Raw bytes → Tokenizer → DOM Tree
  - What: Parse HTML into an internal tree representation
  - How: Use Servo's html5ever crate (already Rust, spec-compliant, used in production by Servo/Firefox)
  - Output: A Document with Element nodes, Text nodes, attributes
  - Key simplification for agents: No need for the full HTML spec error recovery. Parse what's well-formed, skip malformed sections, report parse errors
  structurally.

  2. CSS Engine

  Stylesheets → Parse → Cascade → Computed Styles
  - What: Parse CSS, resolve selectors against the DOM, compute final property values
  - How: Servo's cssparser + selectors crates. cssparser handles tokenization and property parsing. selectors handles matching (descendant, child, sibling,
  pseudo-classes).
  - Key simplification for agents: Only need the properties that affect layout and interaction: display, visibility, position, width, height, z-index,
  opacity, overflow. Colors and fonts are metadata, not rendering concerns. No animations, no transitions, no @media queries for human viewports.

  3. Layout Engine

  DOM + Computed Styles → Layout Tree → Positioned Boxes
  - What: Compute element positions, sizes, visibility, z-ordering
  - How: Custom simplified layout. Block layout (vertical stacking), inline layout (text flow), flexbox basics (enough for modern sites), grid basics.
  Positioned elements (absolute, fixed, sticky).
  - Key simplification for agents: Layout precision can be approximate. An element at (100, 200) vs (102, 198) doesn't matter for clicking. What matters: is
  it visible? Is it interactive? What's its bounding box? What's on top of it? This is the biggest scope reduction vs a human browser.

  4. Network Layer

  URL → DNS → TLS → HTTP → Response Body
  - What: Fetch resources from the web (HTML pages, CSS, images, fonts, scripts)
  - How: reqwest (HTTP) + rustls (TLS) + hickory-resolver (DNS) + custom cookie jar
  - Key difference from human browsers: No CORS enforcement (agents don't need same-origin policy). No mixed-content blocking. All requests are same-origin
  from the agent's perspective. Cookie isolation per session (each session gets its own jar).

  5. Custom DOM (Agent DOM, not Browser DOM)

  HTML DOM → Agent DOM (annotated with interaction metadata)
  - What: A DOM that knows it's for agents. Every element carries:
    - interactive: bool — can this be clicked/typed/scrolled?
    - interaction_type: Enum — Click | Input | Select | Submit | None
    - noise_score: f32 — is this an ad, tracker, cookie banner?
    - goal_relevance: Option<f32> — scored during perception
    - visibility: Enum — Visible | Hidden | Occluded | Offscreen
    - css_hints: Vec<String> — classes and IDs for distraction classification
  - How: This replaces what ans-distill currently does as a separate step. Instead of distilling CDP's DOM into a DistilledPage, the native DOM IS the
  distilled view. Distillation becomes a native property, not a transformation.

  6. Rendering/Rasterization Pipeline

  Layout Tree → Display List → Raster → PNG
  - What: Produce bitmap output for the Vision Eye (GPT-4o-mini)
  - How: Minimal rasterizer using tiny-skia or raqote. Fill rectangles, draw text via rusttype or cosmic-text. No gradients, no shadows, no rounded corners,
  no transforms — unless they affect element visibility.
  - Key simplification: The Vision Eye sees a simplified rendering — boxes with labels, not pixel-perfect web pages. This is arguably BETTER for agents than
  real screenshots because it strips visual noise.

  7. JavaScript Engine (THE BIG DECISION)

  There are three defensible paths:

  ┌───────────┬─────────────────────────────────────────────────────────────────┬──────────────────────────────────────────────────────────────────────┐
  │   Path    │                              What                               │                              Trade-off                               │
  ├───────────┼─────────────────────────────────────────────────────────────────┼──────────────────────────────────────────────────────────────────────┤
  │           │                                                                 │ Fast, simple, immune to JS-based tracking. Breaks SPAs (React, Vue), │
  │ A: No JS  │ Static HTML only. Parse, layout, done.                          │  JS-rendered content, login flows, dynamic forms. ~40% of web        │
  │           │                                                                 │ broken.                                                              │
  ├───────────┼─────────────────────────────────────────────────────────────────┼──────────────────────────────────────────────────────────────────────┤
  │ B:        │ Essential scripts only. Execute <script> tags needed for page   │ Most pragmatic. Embed V8 (v8 crate) or QuickJS. Execute enough to    │
  │ Minimal   │ rendering. Skip analytics, ads, third-party. Sandbox per        │ get content rendered, then freeze.                                   │
  │ JS        │ session.                                                        │                                                                      │
  ├───────────┼─────────────────────────────────────────────────────────────────┼──────────────────────────────────────────────────────────────────────┤
  │ C: Full   │ Complete V8 embedding. Full DOM API, events, timers, XHR/fetch. │ Maximum compatibility, maximum complexity. This is essentially       │
  │ JS        │                                                                 │ building a browser.                                                  │
  └───────────┴─────────────────────────────────────────────────────────────────┴──────────────────────────────────────────────────────────────────────┘

  Recommendation: Path B — Minimal JS with QuickJS.

  QuickJS is a small, embeddable ES2020 engine (Rust bindings via quickjs or rquickjs crate). For an agent browser:
  - Execute first-party scripts to render content
  - Skip google-analytics, gtag, fbq, _gaq, etc.
  - Skip XMLHttpRequest and fetch to third-party origins (or stub them)
  - setTimeout/setInterval limited to 5 seconds then frozen
  - No WebSocket, no WebRTC, no ServiceWorker
  - DOM mutations from JS update the Agent DOM directly

  This gives ~85% site compatibility with ~10% of the complexity of a full browser.

  8. Session Isolation (Rendering Sandbox)

  Session = { Renderer, CookieJar, DOM, Layout, Cache, JSContext? }
  - What: Each agent session gets its own isolated rendering context
  - How: A Renderer struct that owns the full pipeline for one session. Multiple sessions = multiple Renderer instances, isolated by construction. No shared
  state between sessions except via explicit goal state.
  - Memory: A renderer for a typical page: ~5-20MB (DOM + layout + cache). 10 concurrent sessions = ~50-200MB. Compare to Chromium: ~100-300MB per tab.

  9. Native Event System

  Agent action → DOM event → handlers → mutations → re-layout → diff → perception
  - What: Click, type, scroll, select, submit — all as native events on the Agent DOM, not CDP commands
  - How: Click at (x,y) → hit-test layout tree → find target element → create synthetic click event → run JS handlers (if JS enabled) → observe DOM
  mutations → run diff → update layout → emit perception

  ---
  Crate-level impact summary

  ┌──────────────────┬─────────────────┬─────────────────────────────────────────────────────┐
  │      Crate       │      Fate       │                        Notes                        │
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  │ ans-proto        │ Stays           │ Same protobuf contract                              │
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  │ ans-core         │ Stays           │ Same shared types                                   │
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  │ ans-daemon       │ Changes         │ Spawns renderers instead of Chromium processes      │
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  │ ans-cdp          │ Deleted         │ Replaced by native renderer                         │
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  │ ans-distill      │ Heavily changed │ Walks native DOM, not CDP — becomes simpler         │
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  │ ans-diff         │ Heavily changed │ Diffs native DOM directly — faster                  │
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  │ ans-immune       │ Changed         │ Scans at parse time, not after CDP                  │
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  │ ans-goal         │ Stays           │ Unchanged                                           │
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  │ ans-signal       │ Stays           │ Unchanged                                           │
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  │ ans-ipc          │ Changed         │ Session manager owns renderers, not CDP connections │
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  │ ans-gateway      │ Stays           │ Unchanged                                           │
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  │ ans-budget       │ Stays           │ Unchanged                                           │
  │ ans-goal         │ Stays           │ Unchanged                                           │
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  │ ans-signal       │ Stays           │ Unchanged                                           │
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  │ ans-ipc          │ Changed         │ Session manager owns renderers, not CDP connections │
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  │ ans-gateway      │ Stays           │ Unchanged                                           │
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  │ ans-storage      │ Stays           │ Unchanged                                           │
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  │ ans-bench        │ Changed         │ Benchmarks target native renderer, not CDP          │
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  │ ans-stealth      │ Deleted         │ Being native IS the stealth                         │
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  │ NEW: ans-html    │ New             │ HTML parser (wraps html5ever)                       │
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  │ NEW: ans-css     │ New             │ CSS parser + cascade (wraps cssparser + selectors)  │
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  │ NEW: ans-layout  │ New             │ Simplified layout engine                            │
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  │ NEW: ans-render  │ New             │ Rasterization (wraps tiny-skia or raqote)           │
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  │ NEW: ans-network │ New             │ HTTP stack with per-session cookie jars             │
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  │ NEW: ans-dom     │ New             │ Agent-native DOM with interaction metadata          │
  ├──────────────────┼─────────────────┼─────────────────────────────────────────────────────┤
  │ NEW: ans-js      │ New             │ Optional minimal JS engine (QuickJS)                │
  └──────────────────┴─────────────────┴─────────────────────────────────────────────────────┘

  Net: 2 deleted, 6 changed, 7 new, 6 unchanged. Total: 19 crates (from 15).

  ---
  The pragmatic path

  This isn't a rewrite — it's replacing one dependency (Chromium CDP) with ~7 new crates that together form a minimal rendering engine. The entire upper
  stack (gateway, signals, goals, decisions, scoring, LLM) is untouched. The MCP tools work identically. External agents see no difference.

  The unbounded risk is JavaScript. Start with Path A (no JS) to get the full pipeline working end-to-end with static sites. Then add QuickJS incrementally.
  Making ans-js an optional crate (#[cfg(feature = "javascript")]) lets you ship without it and add it later without breaking anything.