//! Strongly typed official MCSMP API groups.
//!
//! Every API handle is inexpensive to clone because it holds a clone of the
//! underlying [`crate::Client`]. Handles issue requests over the same
//! multiplexed WebSocket connection.

mod allowlist;
mod bans;
mod gamerules;
mod ip_bans;
mod operators;
mod players;
mod server;
mod server_settings;

pub use allowlist::AllowlistApi;
pub use bans::BansApi;
pub use gamerules::GamerulesApi;
pub use ip_bans::IpBansApi;
pub use operators::OperatorsApi;
pub use players::PlayersApi;
pub use server::ServerApi;
pub use server_settings::ServerSettingsApi;

use serde::de::DeserializeOwned;
use serde::Serialize;
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
