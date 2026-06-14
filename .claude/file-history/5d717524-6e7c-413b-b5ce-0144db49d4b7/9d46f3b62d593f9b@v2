"""Tests for health-check HTTP server."""

import io
import json
from unittest.mock import MagicMock, patch, AsyncMock

import pytest

from ans_nerves.health import HealthHandler


def _make_handler():
    """Create a HealthHandler bypassing __init__/parse_request chain."""
    handler = HealthHandler.__new__(HealthHandler)
    handler.request = MagicMock()
    handler.client_address = ("127.0.0.1", 12345)
    handler.server = MagicMock()
    handler.wfile = io.BytesIO()
    handler.rfile = io.BytesIO()
    handler.send_response = MagicMock()
    handler.send_header = MagicMock()
    handler.end_headers = MagicMock()

    def _track_code(code, *a, **kw):
        handler._response_code = code

    handler.send_response.side_effect = _track_code
    return handler


class TestHealthHandler:
    def test_health_endpoint(self):
        handler = _make_handler()
        handler.path = "/health"
        handler.do_GET()
        handler.wfile.seek(0)
        body = json.loads(handler.wfile.read())
        assert handler._response_code == 200
        assert body == {"status": "ok"}

    def test_unknown_path(self):
        handler = _make_handler()
        handler.path = "/unknown"
        handler.do_GET()
        handler.wfile.seek(0)
        body = json.loads(handler.wfile.read())
        assert handler._response_code == 404
        assert "error" in body

    def test_ready_all_ok(self):
        mock_client = MagicMock()
        mock_client.health = AsyncMock(return_value=True)
        mock_di = MagicMock()
        mock_di.total_records = 42

        with patch("ans_nerves.grpc_client.get_grpc_client", return_value=mock_client):
            with patch("ans_nerves.scoring.intelligence.DecisionIntelligence", return_value=mock_di):
                handler = _make_handler()
                handler.path = "/ready"
                handler.do_GET()
                handler.wfile.seek(0)
                body = json.loads(handler.wfile.read())
                assert body["status"] == "ready"
                assert body["checks"]["grpc"] == "ok"
                assert "records=42" in body["checks"]["lancedb"]

    def test_ready_grpc_down(self):
        mock_client = MagicMock()
        mock_client.health = AsyncMock(side_effect=Exception("connection refused"))
        mock_di = MagicMock()
        mock_di.total_records = 10

        with patch("ans_nerves.grpc_client.get_grpc_client", return_value=mock_client):
            with patch("ans_nerves.scoring.intelligence.DecisionIntelligence", return_value=mock_di):
                handler = _make_handler()
                handler.path = "/ready"
                handler.do_GET()
                handler.wfile.seek(0)
                body = json.loads(handler.wfile.read())
                assert body["status"] == "not_ready"
                assert "connection refused" in body["checks"]["grpc"]

    def test_ready_lancedb_down(self):
        mock_client = MagicMock()
        mock_client.health = AsyncMock(return_value=True)

        with patch("ans_nerves.grpc_client.get_grpc_client", return_value=mock_client):
            with patch("ans_nerves.scoring.intelligence.DecisionIntelligence", side_effect=Exception("no table")):
                handler = _make_handler()
                handler.path = "/ready"
                handler.do_GET()
                handler.wfile.seek(0)
                body = json.loads(handler.wfile.read())
                assert body["status"] == "not_ready"
                assert "no table" in body["checks"]["lancedb"]

    def test_ready_both_down(self):
        mock_client = MagicMock()
        mock_client.health = AsyncMock(side_effect=Exception("connection refused"))

        with patch("ans_nerves.grpc_client.get_grpc_client", return_value=mock_client):
            with patch("ans_nerves.scoring.intelligence.DecisionIntelligence", side_effect=Exception("db error")):
                handler = _make_handler()
                handler.path = "/ready"
                handler.do_GET()
                handler.wfile.seek(0)
                body = json.loads(handler.wfile.read())
                assert body["status"] == "not_ready"
                assert "connection refused" in body["checks"]["grpc"]
                assert "db error" in body["checks"]["lancedb"]

    def test_json_method(self):
        handler = _make_handler()
        handler._json(201, {"created": True})
        handler.wfile.seek(0)
        body = json.loads(handler.wfile.read())
        assert handler._response_code == 201
        assert body == {"created": True}
        handler.send_header.assert_called_with("Content-Type", "application/json")

    def test_log_message_is_suppressed(self):
        handler = _make_handler()
        handler.log_message("GET /health 200 -")  # no-op, should not raise

    def test_response_headers(self):
        handler = _make_handler()
        handler.path = "/health"
        handler.do_GET()
        handler.send_response.assert_called_with(200)

    def test_readiness_status_200(self):
        mock_client = MagicMock()
        mock_client.health = AsyncMock(return_value=True)
        mock_di = MagicMock()
        mock_di.total_records = 1

        with patch("ans_nerves.grpc_client.get_grpc_client", return_value=mock_client):
            with patch("ans_nerves.scoring.intelligence.DecisionIntelligence", return_value=mock_di):
                handler = _make_handler()
                handler.path = "/ready"
                handler.do_GET()
                assert handler._response_code == 200

    def test_readiness_status_503(self):
        mock_client = MagicMock()
        mock_client.health = AsyncMock(side_effect=Exception("down"))

        with patch("ans_nerves.grpc_client.get_grpc_client", return_value=mock_client):
            with patch("ans_nerves.scoring.intelligence.DecisionIntelligence", side_effect=Exception("down")):
                handler = _make_handler()
                handler.path = "/ready"
                handler.do_GET()
                assert handler._response_code == 503
