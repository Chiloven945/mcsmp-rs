use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use crate::{Client, Result};

/// Escape hatch for MCSMP methods outside the crate's typed surface.
///
/// This is useful for `rpc.discover` during early protocol support and for
/// mod-defined namespaces such as `example_mod:status`.
#[derive(Clone, Debug)]
pub struct RawApi {
    client: Client,
}

impl RawApi {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Calls a method and deserializes its result into `R`.
    pub async fn call<P, R>(&self, method: &str, params: P) -> Result<R>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        let value = serde_json::to_value(params)
            .map_err(|error| crate::Error::Serialization(error.to_string()))?;
        let result = self.client.call_value(method, Some(value)).await?;
        serde_json::from_value(result)
            .map_err(|error| crate::Error::Deserialization(error.to_string()))
    }

    /// Calls a method and returns its untyped JSON result.
    pub async fn call_value(&self, method: &str, params: Option<Value>) -> Result<Value> {
        self.client.call_value(method, params).await
    }
}
