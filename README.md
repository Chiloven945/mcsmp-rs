# mcsmp-rs

`mcsmp-rs` is an asynchronous Rust client for the Minecraft Server Management
Protocol (MCSMP). It provides a Tokio WebSocket transport, JSON-RPC 2.0
multiplexing, typed official resource APIs, capability discovery, and typed
notification streams.

## Status

The crate is under active development and has not been released. Public module
paths and APIs may change before the first stable release.

## Layout

- `src/api`: typed server-management operations and raw extension calls.
- `src/capability`: discovery schemas, protocol versions, and invocation policy.
- `src/client`: public client facade, builder configuration, and state.
- `src/events`: notification models, normalization, decoding, and streams.
- `src/model`: request and response data models.
- `src/transport`: WebSocket handshake, JSON-RPC, request dispatch, sessions, and reconnection.

## Development

Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and
`cargo test` before submitting changes.
