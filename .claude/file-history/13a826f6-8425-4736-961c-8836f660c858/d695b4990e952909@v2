---
name: phase8-complete
description: "Layer 1 Gateway done — MCP server, REST API, WebSocket all wired to real gRPC backend, daemon starts both servers concurrently"
metadata: 
  node_type: memory
  type: project
  originSessionId: 13a826f6-8425-4736-961c-8836f660c858
---

Phase 8 (Layer 1 External API Gateway) is complete.

**What was built:**
- MCP server (`crates/ans-gateway/src/mcp.rs`): 10 real tools (create_session, navigate, click, type_text, scroll, screenshot, get_dom, execute_action, check_goal, create_goal) all calling gRPC backend
- REST API (`crates/ans-gateway/src/rest.rs`): 8 endpoints (health, create_session, navigate, execute_action, screenshot, get_dom, get_goal, create_goal) with real gRPC calls
- WebSocket (`crates/ans-gateway/src/ws.rs`): push_event() for daemon-to-client streaming
- Gateway router (`crates/ans-gateway/src/lib.rs`): Axum routing with auth middleware
- Daemon integration (`crates/ans-daemon/src/main.rs`): starts both gRPC (50051) and gateway (50052) via tokio::spawn, graceful shutdown on Ctrl+C

**Verification:** Build clean, clippy clean, 79 tests pass.

**Why:** This is the external API surface — how Claude, ChatGPT, and other AI agents connect to the Agent Nervous System.
