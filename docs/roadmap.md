# mcsmp-rs roadmap and implementation status

`mcsmp-rs` is an asynchronous Rust client for the Minecraft Server Management
Protocol (MCSMP). The project uses Tokio, WebSocket transport, and JSON-RPC 2.0
to provide typed management operations for Minecraft Java Edition dedicated
servers.

This document describes the implementation plan in English and records the
current status. It is a planning document, not a protocol specification. For
current behavior, read the generated rustdoc and
[protocol compatibility](protocol-compatibility.md).

## Design goals

1. **Idiomatic async Rust.** Public network operations are `async`, clients are
   cheap to clone, and notifications are exposed as a stream rather than
   callback-only APIs.
2. **Strong typing without blocking extensions.** Official MCSMP resources use
   typed request and response models; raw JSON-RPC remains available for custom
   namespaces and future protocol additions.
3. **Discovery-first compatibility.** The runtime discovery schema is more
   authoritative than a hard-coded protocol version table.
4. **Safe recovery.** A connection can reconnect, but management requests are
   never replayed automatically.
5. **Secure defaults.** The library supports authenticated `wss://`
   connections, normal certificate validation, explicit Origin configuration,
   and redacted secrets.

## Milestone status

### Milestone 1 — Transport foundation — complete

Implemented:

- authenticated `ws://` and `wss://` WebSocket handshake;
- bearer, browser-subprotocol, and explicit no-auth choices;
- optional Origin header;
- single reader/writer task pair;
- multiplexed JSON-RPC requests with client-generated request IDs;
- response routing, remote errors, timeouts, Ping/Pong, and explicit shutdown;
- raw JSON-RPC API.

### Milestone 2 — Typed core resources — complete

Implemented:

- models for players, bans, IP bans, operators, messages, server status, and
  game settings;
- typed allowlist, user-ban, IP-ban, player, operator, and server APIs;
- serialization fixture tests and mock WebSocket tests.

### Milestone 3 — Settings, gamerules, and discovery — complete

Implemented:

- all documented `minecraft:serversettings` getter/setter pairs;
- typed gamerules plus legacy string preservation;
- `rpc.discover`, `Capabilities`, `Feature`, `ProtocolVersion`, and
  compatibility modes;
- strict discovery preflight for advertised methods.

### Milestone 4 — Events and reconnect — complete

Implemented:

- typed and raw notification subscriptions;
- supported legacy notification-prefix normalization;
- unknown notification fallback;
- capability-gated world-upgrade preview events;
- fixed and exponential reconnect policies;
- no-replay guarantee for pending operations;
- fresh capability discovery after reconnect.

### Milestone 5 — Delivery quality — complete

Implemented:

- unit, fixture, mock-server, and regression tests;
- documentation builds with warnings denied;
- GitHub Actions for formatting, Clippy, tests, documentation, MSRV, audit, and
  dependency policy;
- Cargo Deny policy and Cargo Audit workflow;
- JSON-RPC parser fuzz target;
- README, server configuration, architecture, compatibility, testing, API, and
  error-handling documentation.

## Current source organization

The source tree is organized by responsibility:

```text
src/
├── api/          typed official resources and raw extension API
├── capability/   discovery, versions, inferred features, compatibility policy
├── client/       public client construction and lifecycle state
├── events/       notification normalization, decoding, and streaming
├── model/        serializable protocol request and response models
└── transport/    private WebSocket, JSON-RPC, session, request, reconnect code
```

The crate root re-exports the common public types. Private transport paths are
not part of the supported downstream API.

## Future work

Potential next work items include:

- real-server integration testing against explicitly supplied Minecraft server
  artifacts;
- richer extension-schema helpers on top of `Capabilities::raw_schema`;
- optional metrics and tracing integration;
- more protocol fixtures as upstream MCSMP evolves;
- release stabilization, semantic-versioning policy, and an MSRV support
  window after the API has settled.

## Non-goals

The library does not attempt to:

- configure or launch a Minecraft server;
- bypass TLS certificate validation;
- replay management operations automatically after a disconnect;
- perform Mojang account lookups or validate player existence;
- guarantee persistence semantics for server settings beyond what MCSMP
  acknowledges at runtime.
