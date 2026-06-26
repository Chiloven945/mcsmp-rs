//! Strongly typed official MCSMP API groups.
//!
//! Each handle maps one official `minecraft:*` namespace to idiomatic async
//! Rust methods. Obtain a handle from [`crate::Client`]; doing so is
//! synchronous and inexpensive because the handle only clones shared client
//! state. Awaiting a method sends one JSON-RPC request over the client's active
//! session.
//!
//! Collection mutations return the server's resulting collection snapshot where
//! the protocol provides one. This keeps callers from guessing how the server
//! resolved selectors, deduplicated entries, or applied defaults. Use
//! [`RawApi`] for extension namespaces or newer methods that do not yet have a
//! typed handle.

mod allowlist;
mod bans;
mod gamerules;
mod ip_bans;
mod operators;
mod players;
mod raw;
mod server;
mod server_settings;

pub use allowlist::AllowlistApi;
pub use bans::BansApi;
pub use gamerules::GamerulesApi;
pub use ip_bans::IpBansApi;
pub use operators::OperatorsApi;
pub use players::PlayersApi;
pub use raw::RawApi;
pub use server::ServerApi;
pub use server_settings::ServerSettingsApi;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{Client, Error, Result};

pub(crate) async fn call<T>(client: &Client, method: &str, params: Option<Value>) -> Result<T>
where
    T: DeserializeOwned,
{
    let result = client.call_typed_value(method, params).await?;
    serde_json::from_value(result).map_err(|error| Error::Deserialization(error.to_string()))
}

pub(crate) fn params<T>(value: T) -> Result<Value>
where
    T: Serialize,
{
    serde_json::to_value(value).map_err(|error| Error::Serialization(error.to_string()))
}
