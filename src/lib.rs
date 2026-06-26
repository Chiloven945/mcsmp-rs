//! `mcsmp-rs` is an asynchronous Rust client for the Minecraft Server
//! Management Protocol (MCSMP).
//!
//! This initial release implements the transport foundation: a TLS-capable
//! WebSocket connection, MCSMP authentication headers, multiplexed JSON-RPC
//! 2.0 calls, bounded request timeouts, and raw notifications.
//!
//! # Example
//!
//! ```no_run
//! use mcsmp_rs::{Auth, Client};
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::builder("wss://127.0.0.1:25585".parse()?)
//!     .auth(Auth::bearer("0123456789012345678901234567890123456789")?)
//!     .origin("mcsmp-rs")
//!     .connect()
//!     .await?;
//!
//! let status = client
//!     .raw()
//!     .call_value("minecraft:server/status", None)
//!     .await?;
//! println!("{status}");
//!
//! client.shutdown().await?;
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod auth;
mod client;
mod error;
mod raw;
mod transport;

pub use auth::{Auth, Secret};
pub use client::{Client, ClientBuilder, ConnectionState, Notification, RequestId};
pub use error::{Error, RemoteError, Result};
pub use raw::RawApi;
