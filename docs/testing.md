# Testing and quality gates

The automated test suite uses local Tokio WebSocket servers. It does not
download or start a Minecraft server in normal CI.

## Required commands

```text
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
cargo deny check
cargo audit
```

The main CI workflow runs formatting, Clippy, tests, documentation generation,
and an MSRV compilation check. The security workflow runs Cargo Audit and
Cargo Deny. These workflows use the dependency lockfile; update `Cargo.lock`
whenever a root dependency changes.

## Protocol fixtures

Fixtures are grouped by protocol concern:

- `tests/fixtures/model`: serialized model contracts.
- `tests/fixtures/discovery`: supported and malformed discovery responses.
- `tests/fixtures/notifications`: legacy/current notification names and preview
  world-upgrade events.
- `tests/fixtures/jsonrpc`: valid, remote-error, and malformed JSON-RPC frames.

## Fuzzing

The `jsonrpc_inbound` target feeds arbitrary bytes into the inbound JSON-RPC
parser. A valid parser result is not required; the property under test is that
untrusted inbound data never causes a panic. See [`../fuzz/README.md`](../fuzz/README.md).
