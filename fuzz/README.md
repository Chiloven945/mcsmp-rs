# Fuzzing

This directory contains [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz)
targets. It is intentionally a separate Cargo package so normal library builds
do not depend on `libfuzzer-sys`.

## Prerequisites

Install a nightly-capable fuzzing toolchain and Cargo Fuzz:

```text
rustup toolchain install nightly
cargo +nightly install cargo-fuzz
```

## JSON-RPC inbound parser

From this directory, run:

```text
cargo +nightly fuzz run jsonrpc_inbound
```

The target accepts arbitrary byte sequences, converts them lossily to UTF-8,
and sends them to the library's inbound JSON-RPC parser. The property being
tested is **no panic**: invalid input may return a protocol error, but it must
not terminate the process.

Cargo Fuzz writes generated inputs to `fuzz/corpus` and crash artifacts to
`fuzz/artifacts`; both are ignored by Git. Minimize a crash with:

```text
cargo +nightly fuzz tmin jsonrpc_inbound fuzz/artifacts/jsonrpc_inbound/<artifact>
```

After fixing a crash, add a deterministic regression test or a fixture under
`tests/fixtures/jsonrpc` before deleting the artifact.
