"""Health-check HTTP server using stdlib only.

Endpoints:
  GET /health — liveness (always 200)
  GET /ready  — readiness (checks gRPC daemon + LanceDB)
"""

from __future__ import annotations

import json
from http.server import HTTPServer, BaseHTTPRequestHandler
from typing import Any

from ans_nerves.config import get_config


class HealthHandler(BaseHTTPRequestHandler):
    def log_message(self, fmt: str, *args: Any) -> None:
        pass  # suppress access logs

    def _json(self, status: int, body: dict[str, Any]) -> None:
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(body).encode())

    def do_GET(self) -> None:
        if self.path == "/health":
            self._json(200, {"status": "ok"})
        elif self.path == "/ready":
            self._ready()
        else:
            self._json(404, {"error": "not found"})

    def _ready(self) -> None:
        checks: dict[str, str] = {}

        # Check gRPC daemon
        try:
            from ans_nerves.grpc_client import get_grpc_client
            import asyncio

            async def _check_grpc() -> None:
                client = get_grpc_client()
                try:
                    await asyncio.wait_for(client.health(), timeout=5.0)
                    checks["grpc"] = "ok"
                except Exception as exc:
                    checks["grpc"] = f"error: {exc}"

            asyncio.run(_check_grpc())
        except Exception as exc:
            checks["grpc"] = f"error: {exc}"

        # Check LanceDB store
        try:
            from ans_nerves.scoring.intelligence import DecisionIntelligence
            di = DecisionIntelligence()
            count = di.total_records
            checks["lancedb"] = f"ok (records={count})"
        except Exception as exc:
            checks["lancedb"] = f"error: {exc}"

        all_ok = all(v.startswith("ok") for v in checks.values())
        status_code = 200 if all_ok else 503

        self._json(status_code, {
            "status": "ready" if all_ok else "not_ready",
            "checks": checks,
        })


def run_health_server(host: str = "0.0.0.0", port: int = 8080) -> None:
    """Start the health-check HTTP server (blocking)."""
    config = get_config()
    server = HTTPServer((host, port), HealthHandler)
    print(f"Health server listening on {host}:{port}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        server.shutdown()
