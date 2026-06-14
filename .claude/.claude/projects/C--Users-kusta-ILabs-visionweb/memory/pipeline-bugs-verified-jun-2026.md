---
name: pipeline-bugs-verified-jun-2026
description: "10 code-verified bugs causing ANS task failures — silent success loop, visible_text data gap, verifier blindness, positive reinforcement, vision throttling, selector gaps, hash embeddings"
metadata:
  type: project
  originSessionId: 000cd6bd-4702-41a4-8963-41a7463485c8
---

## 10 Code-Verified Pipeline Bugs — June 12, 2026

Every bug below was confirmed by reading the actual source code. No speculation.

---

### P0-1: `visible_text` never reaches Python eyes (DATA PIPELINE GAP)

- **Rust** `client.rs:250-267`: Extracts `visible_text` from `document.body.innerText` ✓
- **Rust** `backend.rs:72`: Stored in `PageState.visible_text` ✓
- **Rust** `server.rs:277`: Passed through IPC ✓
- **Python** `loop.py:_gather_page_data:582-620`: **NEVER puts `visible_text` in `page_data`** ✗

Result: `page_data.get("visible_text", [])` at `loop.py:552` **always returns `[]`**.
Both GoalVerifierEye and ErrorDetectorEye receive zero page text content.
The data exists on the Rust side but is dropped in the Python gRPC response handling.

### P0-2: Verifier never triggered on silent failures

`loop.py:469`: `action_succeeded = True` — set whenever no exception thrown, regardless of whether
the click actually advanced the goal. A click on the wrong button returns HTTP 200 and counts as "success."

`loop.py:541-544`: `_should_verify` only true when `errors_this_subgoal >= 2` OR `action_type == "done"`.
Silent failures produce 0 errors and planner doesn't know to say "done" — verifier NEVER runs.
Loop spins until max_steps (50).

### P0-3: Wrong actions get positive reinforcement

`loop.py:514`: `goal_advanced = action_succeeded and not exec_error` → `True` for any non-exception action.
`scorer.py:232-233`: `goal_advanced=True` → `goal: 0.7` (high score).
`planner.py:31`: `_WARM_START_MIN_RECORDS = 3` — after 3 records, warm-start kicks in and
LanceDB recommends the wrong actions back to the planner as "best past actions."
Self-reinforcing failure spiral.

### P0-4: Verifier gets no screenshot

`loop.py:546-554`: Verifier call passes `sub_goal_description`, `success_criteria`, `page_url`,
`page_title`, `visible_text` (empty per P0-1), `dom_summary` (JSON structural dump).
**No `screenshot_base64` is passed.**

Combined with P0-1 (empty visible_text), the verifier sees ONLY a JSON structural dump.
Cannot see: success toasts, error banners, page transitions, visual confirmation.

### P1-5: Vision throttled to 33% of steps

`loop.py:337-344`: Vision runs on steps 1,2 + every 3rd (`step_count % 3 == 0`).
Fast mode: vision NEVER runs. Planner model is DeepSeek-V4-Flash (text-only).
Two "Submit" buttons with identical DOM text are indistinguishable without vision.
66%+ of planning decisions made blind.

### P1-6: Rust `build_selector()` ignores `name` attribute

`engine.rs:381-401`: Only uses `id` attribute or `tag.class`. Never uses `name`.
`<input name="email" class="form-control">` → selector is `input.form-control` — identical
for all 8 inputs on a form. Planner CAN see `name` in element dict (loop.py:819) but
the Rust daemon can't resolve `[name="email"]` selectors.

### P1-7: Selector fallback picks `dom_elements[0]`

`loop.py:647-651`: When planned selector not in DOM, falls back to `dom_elements[0]` —
the first element in goal-relevance-sorted list. Arbitrary. Confidence halved but action
still executed. If the element is clickable, it "succeeds" (P0-2).

### P1-8: Hash embedding fallback is semantically random

`embeddings.py:88-101`: SHA-256 of each token → index into 384-dim vector.
Cryptographic hash: "click" and "clicks" → completely different indices → completely different vectors.
LanceDB ANN search returns random results. Planner prompt says "Use them as STRONG hints."
If FastEmbed (ONNX) fails to load, warm-start planner gets garbage.

### P2-9: gRPC failures silently swallowed

`loop.py:593,600,612`: All three gRPC calls wrapped in `except Exception: pass`.
Failed calls → planner receives empty/near-empty page_data with no error indication.

### P2-10: ask_user infinite pause loop

`loop.py:400-410,763`: 600s timeout → resumes with no human input → same page state →
planner says `ask_user` again → `errors_this_subgoal` reset to 0 at line 409 →
error gate at line 569 never triggers.

---

## Root Cause Chain (closed loop, no escape hatch)

```
Wrong selector clicked → gRPC returns 200 (button exists)
  → action_succeeded = True              [loop.py:469]
  → errors_this_subgoal stays at 0       [loop.py:478 never reached]
  → goal_advanced = True                 [loop.py:514]
  → [FAILED] NOT prefixed                [loop.py:493 — only on exception]
  → _should_verify = False               [loop.py:541-544 — 0 errors, not "done"]
  → Verifier NEVER runs                  
  → LanceDB learns action as "good"      [scorer gives goal=0.7]
  → Warm-start recommends same action    [after 3 records]
  → Planner trusts recommendation        ["Use them as STRONG hints"]
  → Repeats wrong action                 
  → Cycle continues × 50 steps → FAILURE
```

## Why the verifier can't save us even when it does run

When verifier is triggered (2+ exceptions or planner says "done"):
- `visible_text` = `[]` (empty — P0-1 data gap)
- `dom_summary` = JSON structural dump (element tags, selectors, attributes)
- NO screenshot (P0-4)
- Verifier model = DeepSeek-V4-Flash (text-only)
- Verifier is structurally incapable of visually confirming completion

## Fix Priority

1. **Fix `visible_text` data gap** — extract `visible_text` from gRPC response in `_gather_page_data`
2. **Add stagnation detection** — if Rust diff shows no change for 2+ consecutive actions, force verification
3. **Pass screenshots to verifier** — include `screenshot_base64` in verifier call
4. **Fix `goal_advanced` signal** — use diff data, not pure exception check
5. **Run vision when stuck** — override throttle when stagnation detected
6. **Add `name` to `build_selector`** — `engine.rs`: return `tag[name="..."]` when name present
7. **Disable warm-start on hash fallback** — fall back to cold-start when embeddings are meaningless
8. **Don't fallback to `dom_elements[0]`** — escalate instead

[[session-state-jun-2026]] [[decision-memory-pipeline]] [[decision-memory-gap]]
