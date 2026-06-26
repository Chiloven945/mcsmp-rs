# Architecture

`mcsmp-rs` is organized by responsibility rather than implementation phase.

- **Client** owns public construction, configuration, and lifecycle access.
- **API** converts typed MCSMP resource calls into JSON-RPC methods.
- **Model** owns protocol data structures and serialization contracts.
- **Capability** owns `rpc.discover`, protocol-version parsing, and discovery-aware invocation policy.
- **Events** owns raw notification normalization, typed decoding, and stream delivery.
- **Transport** owns the WebSocket handshake, JSON-RPC framing, pending request registry, individual sessions, and
  reconnect supervision.

The public crate root re-exports the common types. Internal transport paths are
not part of the public API contract.

## Connection lifecycle

A client has one active WebSocket session. The session has one reader and one
writer task. Requests are registered before being sent and resolved by JSON-RPC
request ID. An unexpected disconnect fails all pending requests; reconnecting
never replays them. A successful reconnect clears stale capabilities and issues
a fresh `rpc.discover` request.
