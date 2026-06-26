# Error handling and recovery

Every fallible public operation in `mcsmp-rs` returns `Result<T, Error>`. This
guide explains what each category means, whether the server may already have
received an operation, and the usual recovery strategy.

## Error categories

| Error                         | Origin                                   | Was the request necessarily sent? | Usual action                                                                   |
|-------------------------------|------------------------------------------|-----------------------------------|--------------------------------------------------------------------------------|
| `Configuration`               | Local builder/model validation           | No                                | Fix endpoint, credential, timeout, queue, or reconnect configuration           |
| `AuthenticationNotConfigured` | Local builder validation                 | No                                | Select bearer, subprotocol, or explicit no-auth mode                           |
| `AuthenticationRejected`      | HTTP/WebSocket handshake                 | No usable session                 | Check secret, authentication form, Origin, and server configuration            |
| `Transport`                   | TCP/TLS/WebSocket                        | Maybe                             | Inspect connection state and reconnect policy                                  |
| `Closed`                      | Local or peer close                      | Maybe                             | Treat operation outcome as unknown; build a new client if needed               |
| `Reconnecting`                | Local policy gate                        | No                                | Wait for `Client::state() == Connected`, then decide whether to issue new work |
| `Timeout`                     | Local request deadline                   | Yes, possibly                     | Treat operation outcome as unknown; query state before retrying                |
| `Protocol`                    | Invalid inbound JSON-RPC/WebSocket frame | Depends                           | Record diagnostics; connection may become unusable                             |
| `Serialization`               | Local parameter encoding                 | No                                | Fix caller model or raw parameter type                                         |
| `Deserialization`             | Result/notification decoding             | Request may have succeeded        | Preserve raw data if available; update model or report protocol mismatch       |
| `DiscoveryRequired`           | Strict local policy                      | No                                | Call `Client::discover()` on the active session                                |
| `UnsupportedMethod`           | Strict local policy                      | No                                | Feature-gate the call or use a compatible server                               |
| `UnsupportedFeature`          | Capability check                         | No request made by helper         | Disable optional behavior or handle a fallback                                 |
| `Remote`                      | Server JSON-RPC response                 | Yes                               | Inspect `RemoteError` code/message/data                                        |

## Timeouts are ambiguous

A timeout only means the local client did not receive a matching JSON-RPC
response before `ClientBuilder::request_timeout` elapsed. It does **not** prove
that the server did not receive or apply the request.

This matters for management operations:

- A timed-out `players().kick(...)` may already have disconnected a player.
- A timed-out `bans().add(...)` may already have applied a ban.
- A timed-out `server().save(...)` may already have started saving.
- A timed-out `server().stop()` may already have begun shutdown.

Query authoritative state when possible before retrying. For example, call
`players().list()` after an ambiguous kick or `bans().list()` after an ambiguous
ban update.

## Disconnects and reconnects

`ReconnectPolicy` can establish a new WebSocket session after an unexpected
disconnect. It never replays pending requests. The old request completes with
its terminal error, and a new request is your explicit application decision.

Capabilities are cleared after reconnect because the new session can expose a
different server or a changed schema. The reconnect supervisor attempts a fresh
`rpc.discover`, but applications should tolerate discovery being unavailable if
the server is still starting or the policy expires.

During `ConnectionState::Reconnecting`, new calls return `Error::Reconnecting`
instead of being queued. This prevents an application from accidentally
treating reconnect as a durable command queue.

## Remote JSON-RPC errors

`Error::Remote(RemoteError)` is returned when the server sends a JSON-RPC error
object. `RemoteError` contains:

- `code`: numeric, potentially standard or server-specific;
- `message`: human-readable diagnostic text; and
- `data`: optional arbitrary JSON.

Do not parse human-readable message text for durable control flow. Prefer
server-provided error codes or extension-specific `data` fields when the
protocol documents them.

```rust
use mcsmp_rs::{Error, Client};

async fn inspect_error(client: Client) {
    match client.server().status().await {
        Err(Error::Remote(remote)) => {
            eprintln!("server error {}: {}", remote.code, remote.message);
            if let Some(data) = remote.data {
                eprintln!("details: {data}");
            }
        }
        Err(error) => eprintln!("local failure: {error}"),
        Ok(status) => println!("{} online", status.online_player_count()),
    }
}
```

## Strict-mode failures

In `CompatibilityMode::Strict`, `Client::discover()` must succeed before
ordinary API or raw calls. A call that violates this rule returns
`Error::DiscoveryRequired` and is never sent.

After discovery, a method not listed by the server returns
`Error::UnsupportedMethod { method }` before it enters the writer queue. This
is useful for validation-heavy applications because it avoids relying on a
server-side "method not found" response.

## Event-stream errors

`EventStreamError` is separate from `Error` because it describes a local event
subscription, not a JSON-RPC request.

- `Lagged { dropped }`: this subscription fell behind the bounded broadcast
  buffer. Re-query state and continue if the application can tolerate a gap.
- `Closed`: no queued events remain and the client event broadcaster ended.

Unknown notifications are not errors. They appear as `Event::Unknown`, which
contains `RawNotification` with the normalized method name and optional JSON
parameters.

## Logging guidance

It is safe to log `Error`, `RemoteError`, request IDs, method names, and
connection state with normal care for operational data. Do not log management
secrets, `Auth` values, or raw headers. `Secret` redacts itself in `Debug`
output, but callers should not rely on that behavior as a replacement for
sensible secret-handling practices.
