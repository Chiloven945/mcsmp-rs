# mcsmp-rs

[![CI](https://github.com/Chiloven945/mcsmp-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/Chiloven945/mcsmp-rs/actions/workflows/ci.yml)
[![Security](https://github.com/Chiloven945/mcsmp-rs/actions/workflows/security.yml/badge.svg)](https://github.com/Chiloven945/mcsmp-rs/actions/workflows/security.yml)
[![docs.rs](https://img.shields.io/docsrs/mcsmp-rs)](https://docs.rs/mcsmp-rs)

`mcsmp-rs` is an asynchronous, Tokio-based Rust client for the Minecraft Server
Management Protocol (MCSMP). MCSMP is a JSON-RPC 2.0 management API exposed
through a WebSocket by supported Minecraft Java Edition dedicated servers.

The crate provides a single multiplexed WebSocket session, strong Rust models
for official `minecraft:*` methods, capability discovery, typed event streams,
safe reconnect behavior, and generic access to extension namespaces.

> **Development status:** this crate is pre-release software. Its API and
> protocol coverage can change before the first stable release. Pin an exact
> version if your project depends on behavior that is still evolving.

## Highlights

- One `Client` can issue multiple concurrent JSON-RPC requests over one
  WebSocket connection.
- Typed APIs for allowlists, player bans, IP bans, online players, operators,
  server status/save/stop/messaging, live settings, and gamerules.
- `rpc.discover` support through `Client::discover()` and a lossless
  `Capabilities` snapshot.
- `EventStream` for strongly typed official notifications, plus raw
  notifications and a raw extension API for custom namespaces.
- Explicit `Strict`, `Compatible`, and `Permissive` compatibility policies.
- Optional reconnect policies that **never replay** management requests.
- TLS-capable `wss://` connections with normal certificate and hostname
  validation.

## Requirements

- Rust **1.85** or newer.
- A Minecraft Java Edition dedicated server that exposes MCSMP.
- Network access to the server's management host and port.
- A configured management secret, unless the target endpoint deliberately
  permits unauthenticated access.
- An allowed `Origin` value when the server has
  `management-server-allowed-origins` configured.

## Installation

Add the crate to `Cargo.toml`:

```toml
[dependencies]
mcsmp-rs = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

The crate uses the import name `mcsmp_rs`:

```rust
use mcsmp_rs::{Auth, Client};
```

## Secure server setup

MCSMP is disabled by default. Configure the dedicated server's
`server.properties` and restart the server before connecting:

```properties
management-server-enabled=true
management-server-host=127.0.0.1
management-server-port=25585
management-server-secret=<40-character-alphanumeric-secret>
management-server-allowed-origins=https://admin.example.invalid
management-server-tls-enabled=true
management-server-tls-keystore=/path/to/management.p12
```

Use a unique secret for each server. Do not commit it to source control or
print it in application logs. `Secret` redacts its `Debug` representation, but
your program is still responsible for protecting the original string.

Use `wss://` in normal deployments. The library does not expose an insecure
"accept any certificate" option. For isolated local development only, you may
explicitly disable TLS server-side and use `ws://`.

See [server configuration](docs/server-configuration.md) for a property-by-
property reference and local development guidance.

## Quick start

```rust
use mcsmp_rs::{Auth, Client, PlayerRef};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder("wss://127.0.0.1:25585".parse()?)
        .auth(Auth::bearer(
            "0123456789012345678901234567890123456789",
        )?)
        .origin("https://admin.example.invalid")
        .connect()
        .await?;

    let status = client.server().status().await?;
    println!(
        "Minecraft {}: started={}, players={}",
        status.version.name,
        status.started,
        status.online_player_count(),
    );

    client.allowlist().add([PlayerRef::by_name("Alex")?]).await?;

    client.shutdown().await?;
    Ok(())
}
```

`Client` is cheap to clone. Every clone shares the same WebSocket session and
can issue requests concurrently:

```rust
use mcsmp_rs::Client;
async fn example(client: Client) -> Result<(), mcsmp_rs::Error> {
let status_request = client.server().status();
let players_request = client.players().list();

let (status, players) = tokio::try_join!(status_request, players_request)?;
println!("{} online", status.online_player_count());
println!("player snapshot has {} entries", players.len());
Ok(())
}
```

## Authentication and Origin

Use `Auth::bearer(secret)` for native applications. It sends:

```text
Authorization: Bearer <secret>
```

Use `Auth::websocket_subprotocol(secret)` only when browser-compatible
handshake behavior is needed. It sends:

```text
Sec-WebSocket-Protocol: minecraft-v1,<secret>
```

The `minecraft-v1` string is a WebSocket authentication convention. It is not
the semantic MCSMP protocol version.

The server can require a specific `Origin` request header. Configure it with
`ClientBuilder::origin`; the string must match one configured by
`management-server-allowed-origins`. The library does not invent an Origin on
your behalf.

## Typed API groups

Every API handle is obtained from `Client`, is cheap to clone, and sends no
network traffic until an async method is awaited.

| Handle                     | Official namespace         | Typical operations                     |
|----------------------------|----------------------------|----------------------------------------|
| `client.allowlist()`       | `minecraft:allowlist`      | List, replace, add, remove, clear      |
| `client.bans()`            | `minecraft:bans`           | List, replace, add, remove, clear      |
| `client.ip_bans()`         | `minecraft:ip_bans`        | List, replace, add, remove, clear      |
| `client.players()`         | `minecraft:players`        | List online players and kick players   |
| `client.operators()`       | `minecraft:operators`      | List, replace, add, remove, clear      |
| `client.server()`          | `minecraft:server`         | Status, save, stop, system messages    |
| `client.server_settings()` | `minecraft:serversettings` | Read and update active server settings |
| `client.gamerules()`       | `minecraft:gamerules`      | List and update gamerules              |
| `client.raw()`             | Any namespace              | Generic JSON-RPC calls for extensions  |

For detailed type and method documentation, open the generated crate docs or
read [the API guide](docs/api-guide.md).

## Capability discovery and compatibility policy

Call `Client::discover()` after connecting whenever your application needs to
know what the active server supports:

```rust
use mcsmp_rs::{Client, Feature};
async fn example(client: Client) -> Result<(), mcsmp_rs::Error> {
let capabilities = client.discover().await?;

if capabilities.supports_feature(Feature::WorldUpgradeNotifications) {
    println!("world-upgrade notifications are available");
}

if capabilities.supports_method("minecraft:server/status") {
    println!("status endpoint is advertised");
}
Ok(())
}
```

The discovery result retains the original JSON payload in
`Capabilities::raw_schema`, preserving unknown extension fields.

`CompatibilityMode` controls whether discovery restricts outgoing calls:

| Mode                     | Method behavior                                                    | Legacy notification names | Appropriate use                                     |
|--------------------------|--------------------------------------------------------------------|---------------------------|-----------------------------------------------------|
| `Strict`                 | Requires discovery; rejects unadvertised methods locally           | Rejected                  | Validation tools and tightly controlled deployments |
| `Compatible` *(default)* | Allows official and extension calls; discovery remains informative | Normalized                | Most applications                                   |
| `Permissive`             | Does not preflight calls against discovery                         | Normalized                | Experiments and custom/proxy servers                |

See [protocol compatibility](docs/protocol-compatibility.md) for the generation
matrix and forward-compatibility rules.

## Events

Subscribe before performing operations when your program needs to observe
server state changes:

```rust
use mcsmp_rs::{Client, Event, EventStreamError};

async fn watch(client: Client) {
    let mut events = client.subscribe();

    loop {
        match events.recv().await {
            Ok(Event::PlayerJoined { player }) => {
                println!("{player} joined");
            }
            Ok(Event::ServerStopping) => {
                println!("server is stopping");
            }
            Ok(Event::Unknown(raw)) => {
                println!("unmodeled notification: {}", raw.method);
            }
            Ok(_) => {}
            Err(EventStreamError::Lagged { dropped }) => {
                eprintln!("missed {dropped} events; re-query server state");
            }
            Err(EventStreamError::Closed) => break,
            Err(_) => break,
        }
    }
}
```

Event subscriptions are backed by a bounded broadcast channel. A slow
subscriber receives `EventStreamError::Lagged`; query the relevant typed API to
rebuild local state rather than assuming event history is complete. Unknown or
future notifications remain available as `Event::Unknown` rather than closing
the connection.

## Reconnection and safe retries

The default `ReconnectPolicy::Never` leaves a client closed after an unexpected
disconnect. You may configure fixed or exponential retry behavior:

```rust
use std::time::Duration;
use mcsmp_rs::{Auth, Client, ReconnectPolicy};

async fn example() -> Result<(), Box<dyn std::error::Error>> {
let client = Client::builder("wss://127.0.0.1:25585".parse()?)
    .auth(Auth::bearer("0123456789012345678901234567890123456789")?)
    .reconnect_policy(ReconnectPolicy::exponential(
        Duration::from_millis(250),
        Duration::from_secs(10),
        Some(10),
    ))
    .connect()
    .await?;
client.shutdown().await?;
Ok(())
}
```

The library never automatically replays a request after a disconnect. A timeout
or `Error::Closed` does **not** prove that the server did not process the
request. This is particularly important for bans, kicks, saving, and server
shutdown. After reconnecting, inspect authoritative server state and make an
explicit application-level retry decision.

## Raw extension methods

Use `client.raw()` for a mod-defined or future method without a typed Rust
handle:

```rust
use serde::Deserialize;
use serde_json::json;
use mcsmp_rs::Client;

#[derive(Debug, Deserialize)]
struct MaintenanceStatus {
    maintenance: bool,
}

async fn query_extension(client: Client) -> Result<MaintenanceStatus, mcsmp_rs::Error> {
    client
        .raw()
        .call("example_mod:maintenance/status", json!({}))
        .await
}
```

Raw calls use the same timeout, WebSocket, strict discovery policy, and error
model as typed calls.

## Error handling

Every fallible operation returns `mcsmp_rs::Result<T>`. Common recovery paths:

- `Error::Remote`: Minecraft returned a JSON-RPC error; inspect `code`,
  `message`, and optional JSON `data`.
- `Error::Timeout`: the client did not receive a response in time. The server
  might still have applied the action.
- `Error::Closed` / `Error::Reconnecting`: the session is unavailable.
- `Error::DiscoveryRequired` / `Error::UnsupportedMethod`: strict discovery
  policy rejected the call before it was sent.
- `Error::Deserialization`: a server response did not match the expected
  MCSMP model.

See [error handling](docs/error-handling.md) for a fuller recovery guide.

## Further documentation

- [API guide](docs/api-guide.md)
- [Server configuration](docs/server-configuration.md)
- [Protocol compatibility](docs/protocol-compatibility.md)
- [Error handling](docs/error-handling.md)
- [Architecture](docs/architecture.md)
- [Testing and quality gates](docs/testing.md)
- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)

## Development

The project requires Rust 1.85 or newer. Run all local quality gates before
opening a pull request:

```text
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
cargo deny check
cargo audit
```

The repository also contains a JSON-RPC parser fuzz target; see
[`fuzz/README.md`](fuzz/README.md).

## License

Licensed under [Apache-2.0](LICENSE).
