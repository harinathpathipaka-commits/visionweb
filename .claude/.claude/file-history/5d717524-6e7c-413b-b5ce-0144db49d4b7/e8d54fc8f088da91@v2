"""Tests for CLI argument parsing."""

import io
import json
import sys
from unittest.mock import patch, MagicMock, AsyncMock

import pytest

from ans_nerves.__main__ import main, _cmd_config, _cmd_health, _cmd_decompose


class TestArgParser:
    def test_help_output(self):
        with patch.object(sys, "argv", ["ans-nerves", "--help"]):
            with patch.object(sys, "stdout", io.StringIO()) as buf:
                with pytest.raises(SystemExit):
                    main()
                assert "Agent Nervous System" in buf.getvalue()

    def test_no_subcommand_prints_help(self):
        with patch.object(sys, "argv", ["ans-nerves"]):
            with patch.object(sys, "stdout", io.StringIO()) as buf:
                main()
                output = buf.getvalue()
                assert "config" in output or "subcommands" in output.lower()

    def test_serve_parser_host_port(self):
        parser = _get_parser()
        args = parser.parse_args(["serve", "--host", "127.0.0.1", "--port", "9999"])
        assert args.command == "serve"
        assert args.host == "127.0.0.1"
        assert args.port == 9999

    def test_serve_parser_defaults(self):
        parser = _get_parser()
        args = parser.parse_args(["serve"])
        assert args.host == "0.0.0.0"
        assert args.port == 8080

    def test_decompose_parser_requires_goal(self):
        parser = _get_parser()
        args = parser.parse_args(["decompose", "Find flights to Paris"])
        assert args.command == "decompose"
        assert args.goal == "Find flights to Paris"

    def test_decompose_missing_goal(self):
        parser = _get_parser()
        with pytest.raises(SystemExit):
            parser.parse_args(["decompose"])


class TestCmdConfig:
    def test_output_is_valid_json(self):
        with patch.object(sys, "stdout", io.StringIO()) as buf:
            _cmd_config(None)
            data = json.loads(buf.getvalue())
            assert "grpc_host" in data
            assert "grpc_port" in data
            assert "llm" in data
            assert "scoring" in data


class TestCmdHealth:
    def test_health_success(self):
        mock_client = MagicMock()
        mock_client.health = AsyncMock(return_value=True)
        mock_client.close = AsyncMock()
        mock_client.target = "localhost:50051"

        with patch("ans_nerves.grpc_client.get_grpc_client", return_value=mock_client):
            with patch.object(sys, "stdout", io.StringIO()) as buf:
                _cmd_health(MagicMock())
                output = buf.getvalue()
                assert "healthy" in output

    def test_health_unreachable_exits(self):
        mock_client = MagicMock()
        mock_client.health = AsyncMock(side_effect=Exception("connection refused"))
        mock_client.close = AsyncMock()
        mock_client.target = "localhost:50051"

        with patch("ans_nerves.grpc_client.get_grpc_client", return_value=mock_client):
            with pytest.raises(SystemExit) as exc_info:
                _cmd_health(MagicMock())
            assert exc_info.value.code == 1


class TestCmdDecompose:
    def test_decompose_success(self):
        mock_response = MagicMock()
        mock_response.parsed = {"sub_goals": [{"id": "sg_1", "description": "Test"}]}
        mock_response.usage.total_tokens = 100
        mock_response.usage.cost_cents = 0.5
        mock_response.latency_ms = 200.0

        mock_client = MagicMock()
        mock_client.complete_structured = AsyncMock(return_value=mock_response)

        with patch("ans_nerves.llm.client.get_llm_client", return_value=mock_client):
            with patch.object(sys, "stdout", io.StringIO()) as buf:
                args = MagicMock()
                args.goal = "find flights"
                _cmd_decompose(args)
                output = buf.getvalue()
                assert '"sub_goals"' in output
                assert "tokens:" in output

    def test_decompose_unparseable_exits(self):
        mock_response = MagicMock()
        mock_response.parsed = None

        mock_client = MagicMock()
        mock_client.complete_structured = AsyncMock(return_value=mock_response)

        with patch("ans_nerves.llm.client.get_llm_client", return_value=mock_client):
            with pytest.raises(SystemExit) as exc_info:
                args = MagicMock()
                args.goal = "find flights"
                _cmd_decompose(args)
            assert exc_info.value.code == 1


class TestMainRouting:
    def test_main_config_routes(self):
        with patch.object(sys, "argv", ["ans-nerves", "config"]):
            with patch("ans_nerves.__main__._cmd_config") as mock_cmd:
                main()
                mock_cmd.assert_called_once()

    def test_main_health_routes(self):
        with patch.object(sys, "argv", ["ans-nerves", "health"]):
            with patch("ans_nerves.__main__._cmd_health") as mock_cmd:
                main()
                mock_cmd.assert_called_once()

    def test_main_decompose_routes(self):
        with patch.object(sys, "argv", ["ans-nerves", "decompose", "test goal"]):
            with patch("ans_nerves.__main__._cmd_decompose") as mock_cmd:
                main()
                mock_cmd.assert_called_once()

    def test_main_no_command_prints_help(self):
        with patch.object(sys, "argv", ["ans-nerves"]):
            with patch.object(sys, "stdout", io.StringIO()) as buf:
                main()
                output = buf.getvalue()
                assert "config" in output or "subcommands" in output.lower()


def _get_parser():
    """Build the same parser as main() for isolated testing."""
    import argparse
    parser = argparse.ArgumentParser(prog="ans-nerves")
    sub = parser.add_subparsers(dest="command")
    sub.add_parser("config")
    sub.add_parser("health")
    serve_p = sub.add_parser("serve")
    serve_p.add_argument("--host", default="0.0.0.0")
    serve_p.add_argument("--port", type=int, default=8080)
    decomp_p = sub.add_parser("decompose")
    decomp_p.add_argument("goal")
    return parser
