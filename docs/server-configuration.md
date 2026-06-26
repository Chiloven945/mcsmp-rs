# Server configuration

MCSMP is a management endpoint exposed by the Java Edition dedicated server.
It is disabled by default. Enable and secure it in `server.properties` before
building a client connection.

## Minimum secure configuration

```properties
management-server-enabled=true
management-server-host=127.0.0.1
management-server-port=25585
management-server-secret=<40-character-alphanumeric-secret>
management-server-allowed-origins=mcsmp-rs
management-server-tls-enabled=true
management-server-tls-keystore=/absolute/or/server-relative/management.p12
```

Use a distinct secret per server. Do not place the secret in source control,
logs, error messages, or telemetry. `Secret` intentionally redacts its `Debug`
representation, but callers should also avoid formatting `Auth` values into
application logs.

## Relevant properties

| Property                                  | Default              | Client implication                                                                   |
|-------------------------------------------|----------------------|--------------------------------------------------------------------------------------|
| `management-server-enabled`               | `false`              | Must be `true` before a connection can be made.                                      |
| `management-server-host`                  | `localhost`          | Forms the host component of the WebSocket URL.                                       |
| `management-server-port`                  | `0`                  | Set a stable port for persistent clients; `0` selects a port at startup.             |
| `management-server-secret`                | Generated when empty | Pass it to `Auth::bearer` or `Auth::websocket_subprotocol`.                          |
| `management-server-allowed-origins`       | Empty                | Include the exact `ClientBuilder::origin` value. An empty value rejects all origins. |
| `management-server-tls-enabled`           | `true`               | Use `wss://` when enabled. `ws://` is appropriate only for isolated development.     |
| `management-server-tls-keystore`          | None                 | PKCS#12 keystore required for TLS.                                                   |
| `management-server-tls-keystore-password` | None                 | Prefer the documented environment-variable or JVM-property secret sources.           |

The server supports Bearer authentication in the HTTP `Authorization` header.
`mcsmp-rs` uses that form through `Auth::bearer`. The alternative
`Sec-WebSocket-Protocol: minecraft-v1,<secret>` form exists for browser WebSocket
clients and is available through `Auth::websocket_subprotocol`.

`minecraft-v1` in the subprotocol header is an authentication convention; it is
not the MCSMP semantic protocol version.

## TLS and local development

`mcsmp-rs` validates certificates and host names for `wss://` connections. It
does not offer an insecure "accept any certificate" switch. For local testing,
install an appropriate test CA or use an intentionally isolated `ws://` endpoint
with `management-server-tls-enabled=false`.

The library does not synthesize an `Origin` header. Supply one explicitly when
the server has configured `management-server-allowed-origins`:

```rust
use mcsmp_rs::{Auth, Client};
async fn example() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder("wss://127.0.0.1:25585".parse()?)
        .auth(Auth::bearer("secret")?)
        .origin("https://admin.example.invalid")
        .connect()
        .await?;
    client.shutdown().await?;
    Ok(())
}
```
