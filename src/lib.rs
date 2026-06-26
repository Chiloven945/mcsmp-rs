//! `mcsmp-rs` is an asynchronous Rust client for the Minecraft Server
//! Management Protocol (MCSMP).
//!
//! The crate uses a single TLS-capable WebSocket connection with multiplexed
//! JSON-RPC 2.0 calls. It provides strong types for the official MCSMP API,
//! runtime capability discovery, and compatibility policies for servers that
//! implement different protocol generations. Use [`RawApi`] for extension
//! namespaces and protocol features not yet covered by a typed API.
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
//!
//! client
//!     .allowlist()
//!     .add([PlayerRef::by_name("Alex")?])
//!     .await?;
//!
//! client.shutdown().await?;
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Strongly typed official MCSMP API handles.
pub mod api;
/// Compatibility policies for capability-aware method invocation.
pub mod compatibility;
/// Capability discovery and MCSMP protocol-version models.
pub mod discovery;
/// Strongly typed MCSMP request and response models.
pub mod model;

mod auth;
mod client;
mod error;
mod raw;
mod transport;

pub use api::{
    AllowlistApi, BansApi, GamerulesApi, IpBansApi, OperatorsApi, PlayersApi, ServerApi,
    ServerSettingsApi,
};
pub use auth::{Auth, Secret};
pub use client::{Client, ClientBuilder, ConnectionState, Notification, RequestId};
pub use compatibility::CompatibilityMode;
pub use discovery::{Capabilities, Feature, ProtocolVersion, ProtocolVersionParseError};
pub use error::{Error, RemoteError, Result};
pub use model::{
    Difficulty, GameMode, GameRuleKind, GameRuleType, GameRuleValue, IncomingIpBan, IpBan,
    KickPlayer, Message, MinecraftVersion, ModelError, Operator, PlayerRef, ServerState,
    SystemMessage, TypedGameRule, UntypedGameRule, UserBan,
};
pub use raw::RawApi;
