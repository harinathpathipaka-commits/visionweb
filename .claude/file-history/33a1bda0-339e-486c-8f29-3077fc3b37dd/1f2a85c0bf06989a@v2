"""Tests for Goal Decomposer."""

from unittest.mock import MagicMock, AsyncMock, patch

import pytest

from ans_nerves.decomposer.decomposer import GoalDecomposer, GoalSpec, SubGoal


class TestGoalDecomposer:
    def setup_method(self):
        GoalDecomposer._cache.clear()
        self.decomposer = GoalDecomposer()

    @pytest.mark.asyncio
    async def test_empty_goal(self):
        result = await self.decomposer.decompose("")
        assert result.description == ""
        assert len(result.sub_goals) == 0

    @pytest.mark.asyncio
    async def test_whitespace_goal(self):
        result = await self.decomposer.decompose("   ")
        assert result.description == "   "
        assert len(result.sub_goals) == 0

    @pytest.mark.asyncio
    async def test_with_context(self):
        """Empty goal with context still returns early."""
        result = await self.decomposer.decompose("", {"budget": 100})
        assert result.description == ""

    @pytest.mark.asyncio
    async def test_decompose_llm_success(self):
        """LLM successfully returns sub-goals."""
        mock_response = MagicMock()
        mock_response.parsed = {
            "sub_goals": [
                {"id": "sg_1", "description": "Navigate to homepage",
                 "success_criteria": ["URL matches"], "depends_on": []},
                {"id": "sg_2", "description": "Search for flights",
                 "success_criteria": ["results visible"], "depends_on": ["sg_1"]},
            ],
            "estimated_steps": 3,
            "risk_factors": ["CAPTCHA"],
        }

        mock_client = MagicMock()
        mock_client.complete_structured = AsyncMock(return_value=mock_response)

        with patch("ans_nerves.decomposer.decomposer.get_llm_client", return_value=mock_client):
            result = await self.decomposer.decompose("Find flights to Paris")
            assert result.description == "Find flights to Paris"
            assert len(result.sub_goals) == 2
            assert result.sub_goals[0].id == "sg_1"
            assert result.estimated_steps == 3
            assert "CAPTCHA" in result.risk_factors

    @pytest.mark.asyncio
    async def test_decompose_llm_exception_falls_back(self):
        """LLM exception returns GoalSpec with risk factor."""
        mock_client = MagicMock()
        mock_client.complete_structured = AsyncMock(side_effect=Exception("API error"))

        with patch("ans_nerves.decomposer.decomposer.get_llm_client", return_value=mock_client):
            result = await self.decomposer.decompose("Find flights to Paris")
            assert result.description == "Find flights to Paris"
            assert len(result.sub_goals) == 0
            assert "LLM decomposition failed" in result.risk_factors

    @pytest.mark.asyncio
    async def test_decompose_llm_none_parsed_falls_back(self):
        """LLM returns None parsed — falls back with risk factor."""
        mock_response = MagicMock()
        mock_response.parsed = None

        mock_client = MagicMock()
        mock_client.complete_structured = AsyncMock(return_value=mock_response)

        with patch("ans_nerves.decomposer.decomposer.get_llm_client", return_value=mock_client):
            result = await self.decomposer.decompose("Find flights to Paris")
            assert result.description == "Find flights to Paris"
            assert len(result.sub_goals) == 0
            assert "LLM returned unparseable" in result.risk_factors[0]


class TestGoalSpec:
    def test_defaults(self):
        gs = GoalSpec(description="Find flights")
        assert gs.description == "Find flights"
        assert gs.sub_goals == []
        assert gs.max_budget_cents == 500
        assert gs.max_steps == 50

    def test_with_sub_goals(self):
        sg = SubGoal(id="sg_1", description="Navigate to homepage",
                     success_criteria=["URL matches"], depends_on=[])
        gs = GoalSpec(description="Search", sub_goals=[sg],
                      estimated_steps=3, risk_factors=["CAPTCHA"])
        assert len(gs.sub_goals) == 1
        assert gs.estimated_steps == 3
        assert "CAPTCHA" in gs.risk_factors


class TestSubGoal:
    def test_defaults(self):
        sg = SubGoal(id="sg_1", description="Do something")
        assert sg.id == "sg_1"
        assert sg.status == "pending"
        assert sg.success_criteria == []
        assert sg.depends_on == []

    def test_with_dependencies(self):
        sg = SubGoal(
            id="sg_2", description="Fill search form",
            success_criteria=["form submitted"],
            depends_on=["sg_1"],
        )
        assert "sg_1" in sg.depends_on
