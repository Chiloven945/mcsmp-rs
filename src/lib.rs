//! `mcsmp-rs` is an asynchronous Rust client for the Minecraft Server
//! Management Protocol (MCSMP).
//!
//! The crate uses a single TLS-capable WebSocket connection with multiplexed
//! JSON-RPC 2.0 calls. Milestone 2 provides strong types and official API
//! groups for allowlists, player and IP bans, players, operators, and server
//! lifecycle or messaging operations. Use [`RawApi`] for extension namespaces
//! and protocol features not yet covered by a typed API.
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
/// Strongly typed MCSMP request and response models.
pub mod model;

mod auth;
mod client;
mod error;
mod raw;
mod transport;

pub use api::{AllowlistApi, BansApi, IpBansApi, OperatorsApi, PlayersApi, ServerApi};
pub use auth::{Auth, Secret};
pub use client::{Client, ClientBuilder, ConnectionState, Notification, RequestId};
pub use error::{Error, RemoteError, Result};
pub use model::{
    IncomingIpBan, IpBan, KickPlayer, Message, MinecraftVersion, ModelError, Operator, PlayerRef,
    ServerState, SystemMessage, UserBan,
};
pub use raw::RawApi;
