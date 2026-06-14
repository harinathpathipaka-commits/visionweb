# Walkthrough: Agent Decision Layer Fixes

## Summary

Fixed 6 structural bugs in the VisionWeb agent's decision layer that caused it to click "Sign in with Google" instead of form fields, put names in email fields, and loop infinitely on failed actions. All changes are Python-side only — no Rust recompilation needed.

## Files Changed

### [loop.py](file:///d:/ILabs/visionweb/nerves/ans_nerves/planner/loop.py) — 7 changes

| Change | Lines | What |
|--------|-------|------|
| `LoopState.failed_action_keys` | ~73 | New `set` field tracking `(action_type, selector)` pairs that already failed |
| Hard Deduplication Gate | ~600-640 | Before execution, checks if the planned action already failed. If yes, re-plans with `vision_target=None` (disabling Vision override). Max 2 re-plan attempts. |
| Third-party auth redirect detection | ~665-730 | Now fires on ALL click actions (not just when `expected_outcome` exists). Covers Google, Facebook, GitHub, Apple, Microsoft. Auto-navigates back to the original page. |
| Failed action recording | ~705-715 | Records `(action_type, selector)` in `failed_action_keys` set when action fails |
| Safe selector fallback | ~990-1010 | Replaced blind `dom_elements[0]` fallback with fuzzy substring matching. Raises `ValueError` if no match, triggering proper error handling instead of random clicks. |
| `_extract_interactive_elements()` | ~1155-1305 | Added `current_value`, `placeholder` passthrough. Tags social login elements with `[SOCIAL_LOGIN]`. Sorts by DOM order (element_index) instead of relevance score. |
| `_is_social_login_element()` | ~1333-1375 | New utility detecting social login buttons by text patterns ("Sign in with Google", etc.) and provider names in CSS selectors |
| `_extract_vision_target()` | ~1400-1415 | Added social login guard — Vision Eye can no longer force the planner to click social login buttons |

### [prompts.py](file:///d:/ILabs/visionweb/nerves/ans_nerves/llm/prompts.py) — 1 change

| Change | Lines | What |
|--------|-------|------|
| Element attribute exposure | ~655-665 | Added `autocomplete`, `placeholder`, and `current_value` to the interactive element listing shown to the planner LLM. Now the LLM can see `autocomplete=email` to distinguish email from name fields. |

### [planner.py](file:///d:/ILabs/visionweb/nerves/ans_nerves/planner/planner.py) — 1 change

| Change | Lines | What |
|--------|-------|------|
| Duplicate decorator | 37 | Removed duplicate `@dataclass` on `VisionConfirmedTarget` |

### [test_loop.py](file:///d:/ILabs/visionweb/nerves/tests/test_loop.py) — 2 fixes

| Change | What |
|--------|------|
| `test_max_steps_gates_loop` | Added failing verifier mock so the loop actually runs to max_steps instead of exiting on step 1 |
| `test_escalation_on_action_failure` | Added `decompose_single_sub_goal` AsyncMock for the re-decomposition path that now fires correctly thanks to hard dedup |

## How The Fixes Work Together

```
User says: "Sign up on layerinfinite.app with email opus4.6test1@gmail.com"

BEFORE (broken):
  1. Vision Eye sees "Sign in with Google" (big colorful button) → confidence 0.85
  2. Prompt says: "You MUST use this element" → LLM clicks Google
  3. Redirected to accounts.google.com
  4. Marked [FAILED] (soft text only)
  5. Next step: Vision again picks Google → MUST directive wins → clicks again
  6. Infinite loop ♾️

AFTER (fixed):
  1. Vision Eye sees "Sign in with Google" → _is_social_login_element() → SKIP ✅
  2. Vision picks next best element (the email input field)
  3. Planner sees elements in DOM order with autocomplete=email → fills correctly ✅
  4. If anything fails, hard dedup blocks retry + re-plans with vision_target=None ✅
  5. If accidentally redirected to Google, auto-navigates back ✅
```

## Test Results

```
52 passed in 46.57s ✅
```

All existing tests pass. The hard dedup, fuzzy selector matching, and social login guard are all exercised by existing test scenarios (visible in test logs).
