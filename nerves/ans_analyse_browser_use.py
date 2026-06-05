"""ANS full intelligence pipeline: browser-use repo analysis.
Feeds WebFetch data through Decomposer -> Eyes -> Coordinator -> Decision.
"""
import asyncio, json, os, sys
from pathlib import Path

# Load .env (standalone scripts don't go through __main__.py)
for candidate in (Path(__file__).resolve().parent / ".env", Path.cwd() / ".env"):
    if candidate.exists():
        with open(candidate) as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith("#") or "=" not in line:
                    continue
                k, _, v = line.partition("=")
                k, v = k.strip(), v.strip().strip("\"'")
                if k and k not in os.environ:
                    os.environ[k] = v
        break

from ans_nerves.llm.client import get_llm_client
from ans_nerves.llm.prompts import (
    DECOMPOSER_SYSTEM, DECOMPOSER_JSON_SCHEMA,
    VISION_SYSTEM, VISION_JSON_SCHEMA,
    VERIFIER_SYSTEM, VERIFIER_JSON_SCHEMA,
    ERROR_DETECTOR_SYSTEM, ERROR_JSON_SCHEMA,
    COORDINATOR_SYSTEM, COORDINATOR_JSON_SCHEMA,
    build_vision_user_prompt, build_verifier_user_prompt,
    build_error_detector_user_prompt, build_coordinator_user_prompt,
)
from ans_nerves.scoring.intelligence import ActionRecord, DecisionIntelligence

# Repo data from WebFetch (what ANS would get from DOM Reader Eye)
REPO_DATA = {
    "title": "GitHub - browser-use/browser-use",
    "language": "Python (98.2%)",
    "license": "MIT",
    "headings": ["browser-use", "Architecture", "Features", "Quick Start", "Cloud", "Examples", "Benchmarks"],
    "components": {
        "agent": "Single Agent class orchestrating LLM-driven browser tasks",
        "browser": "Wraps Playwright for browser automation (local + cloud)",
        "llm": "ChatBrowserUse (proprietary), ChatGoogle, ChatAnthropic, OpenAI, Ollama",
        "tools": "@tools.action() decorator for custom functions",
        "cli": "Interactive CLI: open, click, type, screenshot, state, close",
        "cloud": "Managed stealth browsers, proxy rotation, CAPTCHA handling",
        "skills": "Claude Code skill integration",
    },
    "integrations": "1000+ (Gmail, Slack, Notion) via cloud",
    "benchmarks": "BU Bench v1 - 100 real-world tasks",
    "pricing": "Open source (MIT) + Cloud SaaS",
}

async def main():
    client = get_llm_client()
    di = DecisionIntelligence()
    total_cost = 0.0
    total_tokens = 0

    print("=" * 70)
    print("ANS FULL PIPELINE: browser-use Repo Analysis")
    print(f"Model: {client.model}")
    print("=" * 70)

    # 1. DECOMPOSER
    print("\n[1] DECOMPOSER: Goal -> Sub-goals")
    r = await client.complete_structured(
        system_prompt=DECOMPOSER_SYSTEM,
        user_prompt="Goal: Analyze browser-use repo. Find architecture, features, LLM, cloud, benchmarks, weaknesses.",
        json_schema=DECOMPOSER_JSON_SCHEMA,
    )
    total_cost += r.usage.cost_cents; total_tokens += r.usage.total_tokens
    subs = r.parsed["sub_goals"] if r.parsed else []
    print(f"    {len(subs)} sub-goals | {r.usage.total_tokens}tok | {r.usage.cost_cents:.2f}cents")
    for s in subs:
        print(f"    - {s['description'][:80]}")

    # 2. VISION EYE
    print("\n[2] VISION EYE: Interpreting repo page structure")
    dom_json = json.dumps(REPO_DATA)
    r = await client.complete_structured(
        system_prompt=VISION_SYSTEM,
        user_prompt=build_vision_user_prompt(
            dom_json, goal_context="Analyze browser-use architecture and features",
            page_url="https://github.com/browser-use/browser-use",
        ),
        json_schema=VISION_JSON_SCHEMA,
    )
    total_cost += r.usage.cost_cents; total_tokens += r.usage.total_tokens
    print(f"    Page type: {r.parsed.get('page_type')} | {r.usage.total_tokens}tok | {r.usage.cost_cents:.2f}cents")
    print(f"    Elements: {len(r.parsed.get('visible_elements', []))}, Overlays: {r.parsed.get('overlays')}")
    vision_report = r.parsed

    # 3. GOAL VERIFIER
    print("\n[3] GOAL VERIFIER: Checking architecture info found")
    r = await client.complete_structured(
        system_prompt=VERIFIER_SYSTEM,
        user_prompt=build_verifier_user_prompt(
            sub_goal_description="Extract architecture from README",
            success_criteria=["Agent component found", "Browser component found",
                              "LLM integrations found", "Cloud offering found",
                              "Benchmarks found"],
            page_url="https://github.com/browser-use/browser-use",
            page_title="browser-use",
            visible_text=[
                f"Agent: {REPO_DATA['components']['agent']}",
                f"Browser: {REPO_DATA['components']['browser']}",
                f"LLM: {REPO_DATA['components']['llm']}",
                f"Cloud: {REPO_DATA['components']['cloud']}",
                f"Benchmarks: {REPO_DATA['benchmarks']}",
            ],
        ),
        json_schema=VERIFIER_JSON_SCHEMA,
    )
    total_cost += r.usage.cost_cents; total_tokens += r.usage.total_tokens
    print(f"    Criteria met: {r.parsed.get('criteria_met')} | Confidence: {r.parsed.get('confidence')}")
    print(f"    {r.parsed.get('reasoning', '')[:120]}")
    verifier_report = r.parsed

    # 4. ERROR DETECTOR
    print("\n[4] ERROR DETECTOR: Classifying absence of ANS comparison")
    r = await client.complete_structured(
        system_prompt=ERROR_DETECTOR_SYSTEM,
        user_prompt=build_error_detector_user_prompt(
            action_description="Search README for comparison with ANS",
            error_message="No ANS comparison found - README doesn't mention ANS",
            page_url="https://github.com/browser-use/browser-use",
            page_title="browser-use",
            visible_text=["No 'ANS' found", "No comparison section exists"],
        ),
        json_schema=ERROR_JSON_SCHEMA,
    )
    total_cost += r.usage.cost_cents; total_tokens += r.usage.total_tokens
    print(f"    Failure type: {r.parsed.get('failure_type')} | Should retry: {r.parsed.get('should_retry')}")
    print(f"    Recovery: {r.parsed.get('recovery_actions', [])}")
    error_report = r.parsed

    # 5. CROSS-EYE COORDINATOR
    print("\n[5] COORDINATOR: Synthesizing all eye reports")
    eyes_data = [
        {"eye_name": "dom_reader", "content": {"page_type": "github_repo", "headings": REPO_DATA["headings"], "language": REPO_DATA["language"]}},
        {"eye_name": "vision", "content": vision_report},
        {"eye_name": "goal_verifier", "content": verifier_report},
        {"eye_name": "error_detector", "content": error_report},
    ]
    r = await client.complete_structured(
        system_prompt=COORDINATOR_SYSTEM,
        user_prompt=build_coordinator_user_prompt(
            json.dumps(eyes_data), goal_context="Analyze browser-use repo vs ANS"
        ),
        json_schema=COORDINATOR_JSON_SCHEMA,
    )
    total_cost += r.usage.cost_cents; total_tokens += r.usage.total_tokens
    print(f"    Confidence: {r.parsed.get('confidence')}")
    print(f"    Perception: {r.parsed.get('unified_perception', '')[:200]}")
    print(f"    Alerts: {len(r.parsed.get('alerts', []))}, Contradictions: {len(r.parsed.get('contradictions', []))}")

    # 6. DECISION INTELLIGENCE
    print("\n[6] DECISION INTELLIGENCE: Scoring + storing")
    record = ActionRecord(
        session_id="browser-use-analysis",
        goal_id="repo-analysis",
        action_type="analyze_readme",
        selector="readme",
        tool="browser",
        context_type="github_repo",
        goal_description="Analyze browser-use repo architecture and features",
        page_type="github_repo",
        action_succeeded=True,
        results_produced="Full architecture, features, integrations, benchmarks found",
        goal_advanced=True,
        sub_goal_completed=True,
    )
    rid = di.record_action(record)
    best = di.query_best_actions(
        action_type="analyze_readme",
        goal_description="Analyze browser-use repo architecture and features",
        page_type="github_repo", k=3,
    )
    print(f"    Record: {rid} | Total stored: {di.total_records}")
    for ba in best:
        print(f"    Best: {ba.action_type} score={ba.composite_score:.3f} dist={ba.distance:.3f}")

    # SUMMARY
    print("\n" + "=" * 70)
    print("ANS PIPELINE COMPLETE")
    print(f"  Total tokens: {total_tokens} | Total cost: {total_cost:.2f} cents")
    print(f"  All 6 components executed successfully")
    print("=" * 70)

asyncio.run(main())
