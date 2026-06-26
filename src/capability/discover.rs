//! Capability discovery request orchestration.

use crate::{Client, Result};

use super::Capabilities;

/// Calls `rpc.discover` and converts the returned schema into capabilities.
pub(crate) async fn discover_capabilities(client: &Client) -> Result<Capabilities> {
    let raw_schema = client.call_discovery_value().await?;
    Ok(Capabilities::from_schema(raw_schema))
}
