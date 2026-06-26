# API guide

This guide explains the public `mcsmp-rs` API at an application level. The
generated rustdoc remains the authoritative reference for every type and
method.

## Client lifecycle

Create one `Client` for one MCSMP endpoint. It owns one WebSocket session with
one reader task and one writer task. Clone the client whenever multiple tasks
need access; cloning does not open another socket.

```rust
use mcsmp_rs::{Auth, Client};

async fn example() -> Result<(), Box<dyn std::error::Error>> {
let client = Client::builder("wss://127.0.0.1:25585".parse()?)
    .auth(Auth::bearer("0123456789012345678901234567890123456789")?)
    .origin("https://admin.example.invalid")
    .connect()
    .await?;

// Use typed API handles here.

client.shutdown().await?;
Ok(())
}
```

`Client::shutdown()` is the explicit lifetime boundary. It closes the active
session, stops background tasks, and fails outstanding calls with `Error::Closed`.
A shut-down client cannot be restarted; construct a new client for a new
connection.

## Resource handles

`Client::allowlist()`, `bans()`, `ip_bans()`, `players()`, `operators()`,
`server()`, `server_settings()`, and `gamerules()` return lightweight handles.
They can be created freely and cloned freely. They all operate on the same
underlying session.

Methods that change a collection generally return the complete collection
after Minecraft applied the request. Use that response as the authoritative
replacement for a local cache. Do not infer that each input item became one
result item: Minecraft can normalize selectors, deduplicate values, ignore
unresolvable targets, or apply server-specific rules.

## Player selectors

`PlayerRef` represents the protocol's player selector. Build one by UUID,
name, or both:

```rust
use mcsmp_rs::PlayerRef;
use uuid::Uuid;

let by_uuid = PlayerRef::by_id(Uuid::parse_str(
    "8667ba71-b85a-4004-af54-457a9734eed7",
)?);
let by_name = PlayerRef::by_name("Alex")?;
let both = PlayerRef::both(
    Uuid::parse_str("8667ba71-b85a-4004-af54-457a9734eed7")?,
    "Alex",
)?;
Ok::<(), Box<dyn std::error::Error>>(())
```

Prefer UUIDs when available because names can change. The crate does not call
Mojang services or validate account existence; Minecraft resolves selectors
when handling the MCSMP request.

## Allowlists, bans, and operators

- `AllowlistApi` manages admission selectors. Whether the allowlist actually
  restricts joining is controlled separately by
  `ServerSettingsApi::set_use_allowlist`.
- `BansApi` manages account/user bans represented by `UserBan`.
- `IpBansApi` manages concrete `IpBan` entries and accepts `IncomingIpBan`
  values when the server should resolve a target player's address.
- `OperatorsApi` manages `Operator` records. Permission levels are validated
  locally in the range `0..=4` when using the provided constructors.

Operations such as `add`, `remove`, `set`, `clear`, and `kick` can have
server-visible effects even if the client loses its connection before receiving
a response. Design application-level administration workflows to inspect
server state before deciding whether to retry.

## Server lifecycle

`ServerApi::status()` returns a `ServerState` snapshot. The `started` field may
be `false` even when the management WebSocket itself is healthy: newer MCSMP
generations can expose discovery and status before the game server completes
startup.

`ServerApi::save(flush)` requests saving. The returned boolean means the server
accepted or started the operation; it does not mean a world save has already
finished. Subscribe to `Event::ServerSaving` and `Event::ServerSaved` when
completion matters.

`ServerApi::stop()` asks Minecraft to shut down. The management socket may close
as a direct consequence. Do not automatically retry the operation.

`SystemMessage` supports normal chat-style messages, action-bar overlays, and
targeted recipients. `Message` supports both literal text and Minecraft
translation keys; use translation keys when each recipient's locale should
resolve the final display text.

## Live server settings

`ServerSettingsApi` reads and changes runtime settings in the
`minecraft:serversettings` namespace. The API exposes a getter and setter for
each known setting. Setter methods return the value acknowledged by Minecraft.

The library intentionally avoids guessing numeric ranges for values such as
view distance, player limits, heartbeat interval, and entity broadcast range.
The server is the final source of validation because supported limits can vary
by server version and configuration.

Changing a live setting does not necessarily mean the corresponding
`server.properties` value will be persisted or that a restart is unnecessary.
Treat MCSMP as a runtime management API and follow the server's own
configuration documentation for persistence behavior.

## Gamerules

`GamerulesApi::list()` returns `TypedGameRule` values with a key, a declared
kind, and a scalar value. `GamerulesApi::update()` takes an `UntypedGameRule`
because the request does not include the authoritative declared kind.

Current MCSMP uses native JSON booleans and integers:

```rust
use mcsmp_rs::UntypedGameRule;

let daylight = UntypedGameRule::boolean("doDaylightCycle", false)?;
let tick_speed = UntypedGameRule::integer("randomTickSpeed", 6)?;
Ok::<(), mcsmp_rs::ModelError>(())
```

For compatibility with earlier protocol experiments, `GameRuleValue` also
preserves string values as `LegacyString`. The library will not silently treat
`"true"` as a boolean. A numeric legacy string can be explicitly converted with
`GameRuleValue::parse_integer()`.

## Discovery

Use `Client::discover()` to obtain a `Capabilities` snapshot:

```rust
use mcsmp_rs::Client;
async fn example(client: Client) -> Result<(), mcsmp_rs::Error> {
    let capabilities = client.discover().await?;

    for method in &capabilities.methods {
        println!("method: {method}");
    }

    for notification in &capabilities.notifications {
        println!("notification: {notification}");
    }
    Ok(())
}
```

`Capabilities::raw_schema` preserves unknown fields. This is useful when
integrating custom MCSMP extensions or debugging a new server version.

In `CompatibilityMode::Strict`, discovery is required before ordinary calls.
If the server does not advertise a method, the client returns
`Error::UnsupportedMethod` locally and does not send the request.

## Notifications

Call `Client::subscribe()` to create a typed `EventStream`. Each stream owns a
separate bounded subscription. The library supports the known official
notifications and emits `Event::Unknown(RawNotification)` for future, custom,
malformed, or capability-gated notifications.

A `Lagged` result means your subscriber missed events. The correct recovery is
not to assume that the next event reconstructs the gap; instead, call the
corresponding query endpoint (for example `players().list()`, `bans().list()`,
or `server().status()`) and replace the stale local state.

For an application that needs the raw JSON form of every notification, call
`Client::subscribe_notifications()` and work with Tokio's `broadcast::Receiver`.

## Raw extension calls

`RawApi::call` serializes a caller-owned parameter type and deserializes the
result into a caller-owned result type. `RawApi::call_value` accepts and returns
`serde_json::Value` for maximum flexibility.

Raw calls remain subject to the same timeout, connection state, and strict
discovery policy as typed API calls. In strict mode, `rpc.discover` itself is
permitted before capabilities are cached; other methods must be advertised.

## Concurrency

The client supports concurrent calls. A request ID is assigned before the
request enters the writer queue, and responses are routed to the matching
awaiting task. Applications may use `tokio::try_join!`, task spawning, or other
standard Tokio concurrency patterns.

Do not infer ordering from completion order. JSON-RPC responses may arrive in
any order, especially when different endpoints have different server-side work.

## Recommended shutdown sequence

1. Stop creating new application-level management work.
2. Await important in-flight tasks or record that they may have completed
   ambiguously.
3. Call `Client::shutdown().await`.
4. Drop client and API-handle clones.

This avoids relying on `Drop` for asynchronous shutdown and makes the lifetime
of network resources explicit.
