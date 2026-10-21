# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). The crate is
pre-release software; its API may change before the first stable release.

## [Unreleased]

### Added

- Asynchronous Tokio client for the Minecraft Server Management Protocol (MCSMP) over a single
  multiplexed WebSocket session.
- Typed APIs for allowlists, player bans, IP bans, online players, operators, server status and
  lifecycle operations, server settings, and gamerules.
- `rpc.discover` capability snapshots with strict, compatible, and permissive compatibility
  policies.
- Typed and raw notification streams, including legacy notification-prefix normalization and
  capability-gated world-upgrade events.
- Fixed and exponential reconnect policies with a no-replay guarantee for pending management
  requests.
- Generic JSON-RPC access for extension namespaces through `RawApi`.

### Fixed

- Restored the crate-root `UserBan` re-export.
- Resolved ambiguous `Error` rustdoc links.
- Updated dependencies to address security advisories in `rustls` and a yanked
  `chacha20` release.
- Documented `RUSTSEC-2026-0381` as a false positive: the flagged
  `webpki-roots 0.26.11` package is a semver shim that re-exports the fixed
  `webpki-roots 1.0.10` root set.
- Corrected the MSRV to Rust 1.99.1 and aligned CI and documentation.

[Unreleased]: https://github.com/Chiloven945/mcsmp-rs/commits/master
