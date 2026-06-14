"""Tests for Decision Intelligence layer."""
import pytest
import tempfile

from ans_nerves.scoring.intelligence import (
    DecisionIntelligence, ActionRecord, ScoredAction,
)
from ans_nerves.scoring.store import LanceDBStore


class TestDecisionIntelligence:
    @pytest.mark.asyncio
    async def test_total_records_starts_at_zero(self):
        with tempfile.TemporaryDirectory() as tmp:
            store = LanceDBStore(uri=tmp)
            di = DecisionIntelligence(store=store)
            assert di.total_records == 0

    @pytest.mark.asyncio
    async def test_record_action_returns_id(self):
        with tempfile.TemporaryDirectory() as tmp:
            store = LanceDBStore(uri=tmp)
            di = DecisionIntelligence(store=store)
            record = ActionRecord(
                session_id="s1", goal_id="g1",
                action_type="click", selector="#btn",
                tool="playwright", action_succeeded=True,
                goal_advanced=True,
            )
            rid = await di.record_action(record)
            assert len(rid) == 16

    @pytest.mark.asyncio
    async def test_record_batch_returns_all_ids(self):
        with tempfile.TemporaryDirectory() as tmp:
            store = LanceDBStore(uri=tmp)
            di = DecisionIntelligence(store=store)
            records = [
                ActionRecord(session_id="s1", goal_id="g1",
                             action_type="click", selector="#btn1"),
                ActionRecord(session_id="s1", goal_id="g1",
                             action_type="type", selector="#input"),
            ]
            ids = await di.record_batch(records)
            assert len(ids) == 2
            assert all(len(rid) == 16 for rid in ids)

    @pytest.mark.asyncio
    async def test_query_best_actions_returns_scored_actions(self):
        with tempfile.TemporaryDirectory() as tmp:
            store = LanceDBStore(uri=tmp)
            di = DecisionIntelligence(store=store)
            await di.record_action(ActionRecord(
                session_id="s1", goal_id="g1",
                action_type="click", selector="#search-btn",
                action_succeeded=True, goal_advanced=True,
                goal_description="Find flights",
            ))
            results = await di.query_best_actions(
                action_type="click", goal_description="Find flights", k=3,
            )
            assert isinstance(results, list)
            for r in results:
                assert isinstance(r, ScoredAction)

    @pytest.mark.asyncio
    async def test_query_best_actions_respects_min_score(self):
        with tempfile.TemporaryDirectory() as tmp:
            store = LanceDBStore(uri=tmp)
            di = DecisionIntelligence(store=store)
            results = await di.query_best_actions(
                action_type="nonexistent", k=5, min_score=0.999,
            )
            assert len(results) == 0

    @pytest.mark.asyncio
    async def test_record_action_with_failure(self):
        with tempfile.TemporaryDirectory() as tmp:
            store = LanceDBStore(uri=tmp)
            di = DecisionIntelligence(store=store)
            record = ActionRecord(
                session_id="s1", goal_id="g1",
                action_type="click", selector="#missing",
                action_succeeded=False,
                error_message="element not found",
                goal_advanced=False,
            )
            rid = await di.record_action(record)
            assert len(rid) == 16


class TestActionRecord:
    def test_defaults(self):
        r = ActionRecord()
        assert r.session_id == ""
        assert r.action_type == ""
        assert not r.action_succeeded

    def test_full_record(self):
        r = ActionRecord(
            session_id="s1", goal_id="g1",
            action_type="click", selector="#btn",
            value="submit", tool="playwright",
            context_type="search", goal_description="Find flights",
            page_type="search_form", action_succeeded=True,
            results_produced="search results loaded",
            goal_advanced=True, criterion_met="results_visible",
            execution_time_ms=150,
        )
        assert r.action_type == "click"
        assert r.tool == "playwright"
        assert r.execution_time_ms == 150


class TestScoredAction:
    def test_defaults(self):
        s = ScoredAction()
        assert s.action_type == ""
        assert s.composite_score == 0.0
