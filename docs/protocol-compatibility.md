# Protocol compatibility

MCSMP is evolving. `mcsmp-rs` treats `rpc.discover` as the strongest
description of a running server and uses protocol versions only as additional
feature-inference evidence.

A Minecraft game version returned by `minecraft:server/status` is not the same
as an MCSMP semantic protocol version. `MinecraftVersion` represents the former;
`ProtocolVersion` represents the latter.

## Supported protocol behavior

| MCSMP generation | Notable behavior | `mcsmp-rs` handling |
| --- | --- | --- |
| Initial / 1.0 | Official typed resources and original notification names | Typed APIs support the documented official methods; compatible mode recognizes the historical notification prefix. |
| 1.1 | Server activity notification | Exposed as `Event::ServerActivity` when advertised. |
| 2.0 | Gamerules use native JSON booleans and integers | `GameRuleValue::{Boolean, Integer, LegacyString}` preserves both current and historical scalar forms. |
| 3.0 | Management endpoint can answer discovery/status before game startup finishes | `ServerState::started == false` is a valid status result, not a connection error. |
| 3.1 preview | World-upgrade lifecycle notifications | Typed only when discovery advertises the notification/feature; otherwise exposed as `Event::Unknown`. |

The table describes protocol behavior known to this crate. A server's discovery
schema remains authoritative for a specific method or notification.

## Compatibility modes

`ClientBuilder::compatibility_mode` controls outgoing preflight behavior and
historical notification normalization.

| Mode | Before discovery | After discovery | Historical `notification:*` names | Extension methods |
| --- | --- | --- | --- | --- |
| `Strict` | Ordinary calls return `Error::DiscoveryRequired` | Unadvertised methods return `Error::UnsupportedMethod` locally | Rejected | Allowed only if advertised |
| `Compatible` *(default)* | Calls are allowed | Discovery is informative, not a hard gate | Normalized to `minecraft:notification/*` | Allowed |
| `Permissive` | Calls are allowed | No capability preflight | Normalized to `minecraft:notification/*` | Allowed |

Strict mode is appropriate for validation tools, controlled environments, and
applications that want to prevent accidental calls to unsupported endpoints.
Compatible mode is the best default for ordinary administration clients.
Permissive mode is useful for experimentation with extensions and proxies.

## Discovery schema retention

`Capabilities::from_schema` accepts supported map and array forms for methods
and notifications. The resulting snapshot exposes:

- `protocol_version` when a parseable version is present;
- `methods` and `notifications` as complete names;
- inferred `features`;
- per-entry schema fragments where available; and
- the unmodified `raw_schema`.

The raw schema is important for forward compatibility. `mcsmp-rs` does not
discard unfamiliar extension declarations merely because it cannot yet model
them in a dedicated Rust type.

## Gamerule compatibility

Current MCSMP gamerules are native JSON scalars. Earlier forms may use strings.
The crate preserves strings as `GameRuleValue::LegacyString`.

No implicit boolean conversion is performed:

- `true` becomes `GameRuleValue::Boolean(true)`;
- `12` becomes `GameRuleValue::Integer(12)`;
- `"12"` becomes `GameRuleValue::LegacyString("12".into())`;
- `"true"` remains a legacy string.

Applications that intentionally support legacy numeric strings can use
`GameRuleValue::parse_integer()`. Native values are validated against the
server-declared `GameRuleKind` when decoding `TypedGameRule`.

## Notification compatibility

Known official notifications are normalized and decoded into `Event` values.
The default compatible and permissive modes recognize the historical
`notification:*` prefix and normalize it to the current
`minecraft:notification/*` form.

A future method, a custom mod notification, a malformed payload, or a
capability-gated preview notification becomes `Event::Unknown(RawNotification)`
rather than closing the connection. Applications can inspect the raw method and
JSON parameters while waiting for an updated typed API.

## Reconnect behavior

Reconnect is transport recovery, not request retry. When a session ends
unexpectedly:

1. all pending calls fail;
2. no pending request is sent again;
3. a configured reconnect policy may create a new WebSocket session;
4. stale capabilities are cleared; and
5. the client attempts a fresh `rpc.discover`.

Applications must make an explicit, domain-specific decision about whether a
failed operation can be issued again. This is especially important for
non-idempotent management operations.
