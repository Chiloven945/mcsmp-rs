//! Asynchronous Rust client for the Minecraft Server Management Protocol.
//!
//! `mcsmp-rs` uses one TLS-capable WebSocket session with multiplexed JSON-RPC
//! 2.0 calls. The public surface is organized around typed resource APIs,
//! capability discovery, and asynchronous notifications. Use [`RawApi`] for
//! extension namespaces that do not yet have a typed handle.
//!
//! # Example
//!
//! ```no_run
//! use mcsmp_rs::{Auth, Client, PlayerRef};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::builder("wss://127.0.0.1:25585".parse()?)
//!     .auth(Auth::bearer("0123456789012345678901234567890123456789")?)
//!     .origin("mcsmp-rs")
//!     .connect()
//!     .await?;
//!
//! let status = client.server().status().await?;
//! println!("{} players online", status.online_player_count());
//! client.allowlist().add([PlayerRef::by_name("Alex")?]).await?;
//! client.shutdown().await?;
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Typed official MCSMP resource APIs.
pub mod api;
/// Capability discovery, protocol versions, and invocation policy.
pub mod capability;
/// Client construction and connection state.
pub mod client;
/// Typed notifications and event streams.
pub mod events;
/// Strongly typed MCSMP request and response models.
pub mod model;

mod auth;
mod error;
mod transport;

pub use api::{
    AllowlistApi, BansApi, GamerulesApi, IpBansApi, OperatorsApi, PlayersApi, RawApi, ServerApi,
    ServerSettingsApi,
};
pub use auth::{Auth, Secret};
pub use capability::{
    Capabilities, CompatibilityMode, Feature, ProtocolVersion, ProtocolVersionParseError,
};
pub use client::{Client, ClientBuilder, ConnectionState};
pub use error::{Error, RemoteError, Result};
pub use events::{Event, EventStream, EventStreamError, RawNotification};
pub use model::{
    Difficulty, GameMode, GameRuleKind, GameRuleValue, IncomingIpBan, IpBan, KickPlayer, Message,
    MinecraftVersion, ModelError, Operator, PlayerRef, ServerState, SystemMessage, TypedGameRule,
    UntypedGameRule, UserBan,
};
pub use transport::{ReconnectPolicy, RequestId};
