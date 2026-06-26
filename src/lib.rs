//! # mcsmp-rs
//! 
//! Asynchronous Rust client for the Minecraft Server Management Protocol (MCSMP).
//!
//! `mcsmp-rs` is a Tokio-based client for the management WebSocket exposed by
//! recent Minecraft Java Edition dedicated servers. It speaks JSON-RPC 2.0 over
//! one multiplexed WebSocket connection and offers a strongly typed facade for
//! the official `minecraft:*` method namespaces.
//!
//! ## What this crate provides
//!
//! - A [`Client`] that can issue many concurrent JSON-RPC requests over one
//!   WebSocket session.
//! - Typed resource handles for player management, allowlists, bans, operators,
//!   server lifecycle operations, live settings, and gamerules.
//! - [`Capabilities`] obtained from `rpc.discover`, plus [`CompatibilityMode`]
//!   to control how discovery information constrains requests.
//! - A typed [`EventStream`] for server notifications and
//!   [`RawNotification`] access for extension-defined or future notifications.
//! - [`RawApi`] for MCSMP extensions that are not represented by a typed API
//!   yet.
//!
//! ## Before connecting
//!
//! The Minecraft management endpoint is disabled by default. Configure the
//! dedicated server's `server.properties` and restart the server before using
//! this crate:
//!
//! ```text
//! management-server-enabled=true
//! management-server-host=127.0.0.1
//! management-server-port=25585
//! management-server-secret=<40-character-alphanumeric-secret>
//! management-server-allowed-origins=mcsmp-rs
//! management-server-tls-enabled=true
//! ```
//!
//! Use a `wss://` URL when TLS is enabled. The server compares the `Origin`
//! request header against `management-server-allowed-origins`; configure
//! [`ClientBuilder::origin`] with an allowed value when the server requires
//! one. See `docs/server-configuration.md` in the repository for the full
//! property reference and local-development guidance.
//!
//! ## Quick start
//!
//! ```no_run
//! use mcsmp_rs::{Auth, Client, PlayerRef};
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::builder("wss://127.0.0.1:25585".parse()?)
//!     .auth(Auth::bearer("0123456789012345678901234567890123456789")?)
//!     .origin("mcsmp-rs")
//!     .connect()
//!     .await?;
//!
//! let status = client.server().status().await?;
//! println!(
//!     "Minecraft {} has {} players online",
//!     status.version.name,
//!     status.online_player_count(),
//! );
//!
//! client.allowlist().add([PlayerRef::by_name("Alex")?]).await?;
//! client.shutdown().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Request lifecycle and retries
//!
//! A request completes with a typed result, [`Error::Remote`] for a JSON-RPC
//! error object returned by Minecraft, or a local [`Error`] such as
//! [`Error::Timeout`] or [`Error::Closed`]. The client never automatically
//! replays a request after a connection loss. That rule is intentional:
//! management operations such as banning, kicking, changing settings, and
//! stopping a server may be non-idempotent. When a [`ReconnectPolicy`] is
//! enabled, wait until [`Client::state`] reports `Connected` and deliberately
//! issue a new request only when repeating it is safe for your application.
//!
//! ## Capability discovery
//!
//! Call [`Client::discover`] after connecting when your application needs to
//! inspect server support or when [`CompatibilityMode::Strict`] is selected.
//! Discovery produces a [`Capabilities`] snapshot with advertised methods,
//! notifications, inferred features, and the unmodified server schema. In
//! strict mode, ordinary calls are rejected locally until discovery succeeds,
//! and methods absent from the advertised schema return
//! [`Error::UnsupportedMethod`] rather than being written to the socket.
//!
//! ## Notifications
//!
//! Call [`Client::subscribe`] before performing operations when you need a
//! typed event stream. `EventStream::recv` is convenient for loops that do not
//! otherwise use `Stream`; subscribers that fall behind receive
//! [`EventStreamError::Lagged`] and should query authoritative state again.
//! Unknown extension notifications are surfaced as [`Event::Unknown`] instead
//! of being discarded or closing the connection.
//!
//! ## Crate layout
//!
//! The stable user-facing surface is re-exported from this crate root. The
//! public modules group related APIs:
//!
//! - [`api`] contains typed and raw method handles.
//! - [`capability`] contains discovery, protocol-version, and compatibility
//!   types.
//! - [`client`] contains connection construction and lifecycle state.
//! - [`events`] contains typed notification models and stream behavior.
//! - [`model`] contains serializable request and response types.
//!
//! Internal WebSocket, session, and JSON-RPC implementation details are kept
//! private so applications can use the typed surface without coupling to the
//! transport implementation.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Typed official MCSMP resource APIs and the raw extension API.
pub mod api;
/// Capability discovery, protocol versions, inferred features, and invocation policy.
pub mod capability;
/// Client construction and connection lifecycle state.
pub mod client;
/// Typed MCSMP notifications, raw notifications, and asynchronous event streams.
pub mod events;
/// Strongly typed request and response models used by the MCSMP APIs.
pub mod model;

mod auth;
mod error;
mod transport;

/// Typed handle for `minecraft:allowlist` methods.
pub use api::AllowlistApi;
/// Typed handle for `minecraft:bans` methods.
pub use api::BansApi;
/// Typed handle for `minecraft:gamerules` methods.
pub use api::GamerulesApi;
/// Typed handle for `minecraft:ip_bans` methods.
pub use api::IpBansApi;
/// Typed handle for `minecraft:operators` methods.
pub use api::OperatorsApi;
/// Typed handle for `minecraft:players` methods.
pub use api::PlayersApi;
/// Untyped and generic JSON-RPC access for extension methods.
pub use api::RawApi;
/// Typed handle for `minecraft:server` methods.
pub use api::ServerApi;
/// Typed handle for `minecraft:serversettings` methods.
pub use api::ServerSettingsApi;

/// WebSocket-handshake authentication configuration.
pub use auth::Auth;
/// Validated management-server secret that redacts itself in `Debug` output.
pub use auth::Secret;

/// Parsed `rpc.discover` capability snapshot.
pub use capability::Capabilities;
/// Discovery-aware policy that controls preflight checks and historical forms.
pub use capability::CompatibilityMode;
/// Optional protocol behavior inferred from capability discovery.
pub use capability::Feature;
/// Numeric MCSMP semantic protocol version.
pub use capability::ProtocolVersion;
/// Parse error returned when text is not a valid MCSMP semantic version.
pub use capability::ProtocolVersionParseError;

/// Cloneable asynchronous MCSMP connection facade.
pub use client::Client;
/// Fluent configuration builder used to create a `Client`.
pub use client::ClientBuilder;
/// Current lifecycle state of a `Client`.
pub use client::ConnectionState;

/// Local or remote error returned by client and API operations.
pub use error::Error;
/// JSON-RPC error object returned by the remote server.
pub use error::RemoteError;
/// Convenient result alias whose default error type is `Error`.
pub use error::Result;

/// Strongly typed notification emitted by the management server.
pub use events::Event;
/// Stream of typed notifications produced by `Client::subscribe`.
pub use events::EventStream;
/// Recoverable delivery error produced while consuming an `EventStream`.
pub use events::EventStreamError;
/// Normalized JSON-RPC notification payload for raw event consumers.
pub use events::RawNotification;

/// Server difficulty value used by `ServerSettingsApi`.
pub use model::Difficulty;
/// Default game mode used by `ServerSettingsApi`.
pub use model::GameMode;
/// Declared scalar kind of a gamerule.
pub use model::GameRuleKind;
/// Native or legacy scalar gamerule value.
pub use model::GameRuleValue;
/// Input model for creating an IP ban by address and/or player selector.
pub use model::IncomingIpBan;
/// Resolved IP ban entry.
pub use model::IpBan;
/// Request model for disconnecting a selected player.
pub use model::KickPlayer;
/// Literal or translatable Minecraft display message.
pub use model::Message;
/// Minecraft game version reported by server status.
pub use model::MinecraftVersion;
/// Local validation error returned while constructing protocol models.
pub use model::ModelError;
/// Server operator entry.
pub use model::Operator;
/// UUID and/or name selector for a Minecraft player.
pub use model::PlayerRef;
/// Snapshot of server lifecycle state, online players, and game version.
pub use model::ServerState;
/// Message sent by `ServerApi::system_message`.
pub use model::SystemMessage;
/// Gamerule returned by the server with its declared kind.
pub use model::TypedGameRule;
/// Gamerule update request without a declared kind.
pub use model::UntypedGameRule;

/// Automatic reconnect behavior after an unexpected session interruption.
pub use transport::ReconnectPolicy;
/// Client-generated identifier assigned to one JSON-RPC request.
pub use transport::RequestId;

/// Fuzzing-only entry points used by the repository's local `cargo-fuzz`
/// targets. This module is hidden from generated documentation and is not part
/// of the supported downstream API.
#[cfg(feature = "fuzzing")]
#[doc(hidden)]
pub mod fuzzing {
    /// Parses untrusted inbound JSON-RPC text and deliberately ignores the
    /// result. Fuzz targets assert that this operation never panics.
    pub fn parse_inbound_jsonrpc(input: &str) {
        let _ = crate::transport::fuzz_parse_inbound(input);
    }
}
