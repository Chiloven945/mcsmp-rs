# Contributing to mcsmp-rs

Thank you for contributing to `mcsmp-rs`. The project aims to make MCSMP safe
and ergonomic for async Rust applications while preserving protocol details that
matter for server administration.

## Project principles

- **Discovery is stronger than a version guess.** When a method, notification,
  or feature may vary between server versions, use `rpc.discover` data and
  preserve unknown schema fields.
- **Typed APIs must match the wire contract exactly.** Keep JSON names,
  optional-field behavior, scalar types, and result wrapper fields aligned with
  the protocol fixtures and documented server behavior.
- **Unknown protocol data should remain observable.** Prefer raw schema,
  `RawApi`, `RawNotification`, or `Event::Unknown` over discarding extension or
  future fields.
- **Never replay management requests automatically.** A disconnected request
  might already have changed server state. Reconnect may create a new session,
  but it must not re-send in-flight calls.
- **Public documentation is part of the API.** Every public type, field,
  variant, and callable method needs clear English rustdoc explaining purpose,
  wire semantics, validation, failure modes, and recovery expectations.

## Repository layout

- `src/api`: typed official MCSMP method groups and raw extension access.
- `src/capability`: discovery, versions, inferred features, and compatibility
  policy.
- `src/client`: public construction and lifecycle state.
- `src/events`: normalization, typed decoding, and stream behavior.
- `src/model`: request/response data models and serialization validation.
- `src/transport`: private JSON-RPC, WebSocket, session, request, and
  reconnect implementation.
- `tests/fixtures`: protocol JSON organized by models, discovery,
  notifications, and JSON-RPC.
- `docs`: user and maintainer documentation.
- `fuzz`: parser fuzz targets.

Do not introduce development-phase names such as `milestone_four.rs` for
long-lived source or test files. Name files for their protocol concern or
observable behavior.

## Development environment

Install Rust 1.85 or newer and clone the repository. The normal test suite uses
local Tokio WebSocket mock servers; it does not download or launch Minecraft.

Run every quality gate before opening a pull request:

```text
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
cargo deny check
cargo audit
```

The documentation command is intentionally strict. Broken intra-doc links,
missing public rustdoc, and documentation warnings should be fixed rather than
suppressed.

## Tests and fixtures

Choose the test location by behavior:

- `tests/models.rs`: model construction and serialization;
- `tests/api_resources.rs`: typed resource method names and payloads;
- `tests/server_settings.rs`: live settings methods;
- `tests/gamerules.rs`: scalar compatibility and update contracts;
- `tests/capabilities.rs`: discovery and strict policy;
- `tests/events.rs`: notification normalization, decoding, and delivery;
- `tests/reconnect.rs`: reconnect safety behavior;
- `tests/transport.rs`: JSON-RPC and WebSocket transport behavior.

When protocol behavior changes, add a minimal fixture under the appropriate
`tests/fixtures` category and a deterministic regression test. Use mock
WebSocket tests to assert that a typed method sends the correct method name,
parameters, and response handling.

## Public API changes

For any public API addition or behavior change:

1. Add or update rustdoc with at least one concise usage or semantic example
   when the feature is non-obvious.
2. Update the README or an appropriate page in `docs/`.
3. Add tests for serialization, deserialization, or observable session behavior.
4. Update `CHANGELOG.md`.
5. Consider `CompatibilityMode`, `Capabilities`, legacy protocol forms, raw
   extension fallbacks, and reconnect semantics.

## Dependency changes

Keep dependencies minimal and intentional. When adding or updating a root
dependency, update `Cargo.lock` and verify both `cargo deny check` and
`cargo audit`. Do not add advisories or license exceptions without a documented
reason and removal condition.

## Fuzz regressions

The inbound JSON-RPC parser handles untrusted network input. When fuzzing finds
a panic or pathological input:

1. reduce it to the smallest deterministic example;
2. add it as a test or fixture first;
3. fix the parser so it returns an error or a raw/unknown event instead of
   panicking; and
4. retain the fuzz corpus entry when it adds coverage.

See [testing and quality gates](docs/testing.md) and
[`fuzz/README.md`](fuzz/README.md) for more detail.
