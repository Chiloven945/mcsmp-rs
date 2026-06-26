# Architecture

`mcsmp-rs` is organized by protocol responsibility rather than by development
milestone. The layout separates the stable user-facing API from private
transport machinery so applications do not need to depend on WebSocket task
details.

## Public layers

### Client

`src/client` contains:

- `Client`: a cloneable facade around one shared session;
- `ClientBuilder`: endpoint, authentication, Origin, timeout, compatibility,
  queue-capacity, and reconnect configuration; and
- `ConnectionState`: an advisory lifecycle snapshot.

A client clone shares one active session. It is not another connection.

### API

`src/api` maps official MCSMP methods to strongly typed Rust handles:

- `allowlist`, `bans`, `ip_bans`, `players`, and `operators` manage player
  administration;
- `server` manages status, saves, graceful shutdown, and messages;
- `server_settings` manages live runtime settings;
- `gamerules` manages typed and legacy gamerule values; and
- `raw` exposes extension-defined JSON-RPC methods.

The API layer serializes request parameters, calls the client, and deserializes
the documented result field. It does not own WebSocket state.

### Model

`src/model` contains protocol data structures and wire contracts. Model
constructors validate invariants that are stable across server versions, such
as non-blank selectors and operator permission levels. They intentionally do
not guess server-dependent numeric bounds, account existence, timestamp syntax,
or persistence behavior.

### Capability

`src/capability` owns `rpc.discover` parsing, semantic protocol versions,
inferred feature flags, and compatibility policy. Discovery data is retained
losslessly so applications can inspect future or extension-specific schema
members.

### Events

`src/events` normalizes supported historical notification names, decodes known
official payloads, retains unknown payloads as raw notifications, and provides
a per-subscriber typed stream.

### Transport

`src/transport` is private implementation machinery:

- `websocket` builds the authenticated handshake and opens the socket;
- `jsonrpc` serializes requests and parses untrusted inbound frames;
- `request` assigns identifiers and tracks pending response waiters;
- `session` owns the reader/writer task pair for one connection; and
- `reconnect` supervises optional reconnection after unexpected failure.

## One-session request flow

1. A typed or raw API call constructs a method name and optional JSON value.
2. The client applies strict capability checks when configured.
3. The session allocates a non-zero `RequestId` and stores a one-shot response
   waiter before enqueueing the outbound message.
4. The single writer task serializes and sends the JSON-RPC request.
5. The single reader task parses incoming text frames.
6. A response with an ID resolves the matching waiter; a notification is
   published to raw and typed event subscribers.
7. The calling future applies its configured timeout and deserializes the JSON
   result into its requested type.

Responses can arrive out of order. The request ID registry, not arrival order,
determines which future receives each response.

## Session failure and reconnect

An unexpected close or transport failure immediately fails all pending
requests. The library never replays them. This is a deliberate safety property:
a response may be lost after a server already executed a kick, ban, save,
setting update, or stop request.

When `ReconnectPolicy` is enabled, the client enters `Reconnecting`, waits
according to the configured schedule, opens a fresh WebSocket, clears stale
capabilities, and attempts discovery for the new session. New calls are not
queued during this period; they return `Error::Reconnecting`.

Explicit `Client::shutdown()` is different: it ends the client permanently and
does not start a reconnect supervisor.

## Notification delivery

The transport publishes each incoming notification to bounded Tokio broadcast
channels. Every `Client::subscribe()` call gets an independent receiver. A
slow receiver can lag and miss historical events; it receives
`EventStreamError::Lagged { dropped }` and should query authoritative state.

The event layer does not treat an unknown method or malformed known payload as
a session failure. It exposes `Event::Unknown(RawNotification)` instead. This
allows applications to observe new protocol features or mod-defined events
without losing the entire connection.

## Security boundaries

The JSON-RPC parser treats inbound text as untrusted. Malformed frames are
reported as protocol errors and must not panic the process. Tests and the local
fuzz target cover this boundary.

Secrets are validated for header safety and redacted in `Debug` output. TLS
certificate and hostname validation are left enabled for `wss://` connections;
the public API intentionally does not expose an insecure certificate-bypass
switch.

## Public API stability

The crate is pre-release. The crate root re-exports the intended user-facing
types so common imports remain concise. Private transport module paths are not
part of the supported downstream API and can change without notice.
