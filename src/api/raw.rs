//! Generic access to JSON-RPC methods outside the typed API surface.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{Client, Result};

/// Generic JSON-RPC access for extension-defined or not-yet-modeled methods.
///
/// Obtain this handle from [`crate::Client::raw`]. It shares the client's
/// connection, timeout, discovery policy, and reconnect behavior. It is
/// intended for MCSMP extensions such as mod-defined namespaces, not as a
/// replacement for the typed handles in [`crate::api`].
///
/// In [`crate::CompatibilityMode::Strict`] mode, calls other than
/// `rpc.discover` are still checked against the discovered method list. Use
/// [`Self::call_value`] when an extension result has no stable Rust model yet,
/// and [`Self::call`] when a caller-owned type can deserialize the result.
#[derive(Clone, Debug)]
pub struct RawApi {
    client: Client,
}

impl RawApi {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Calls `method` with serializable parameters and deserializes the result.
    ///
    /// `method` must be a complete JSON-RPC method name such as
    /// `example_mod:maintenance/status`. `params` is serialized as one JSON
    /// value, so callers can provide a struct, map, array, scalar, or
    /// `serde_json::Value`.
    ///
    /// The method returns [`crate::Error::Serialization`] before a request is
    /// sent when `params` cannot be serialized, and
    /// [`crate::Error::Deserialization`] when the remote result cannot be
    /// decoded as `R`. Remote JSON-RPC failures are returned as
    /// [`crate::Error::Remote`].
    pub async fn call<P, R>(&self, method: &str, params: P) -> Result<R>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        let value = serde_json::to_value(params)
            .map_err(|error| crate::Error::Serialization(error.to_string()))?;
        let result = self.client.call_raw_value(method, Some(value)).await?;
        serde_json::from_value(result)
            .map_err(|error| crate::Error::Deserialization(error.to_string()))
    }

    /// Calls `method` and returns its untyped JSON result.
    ///
    /// Pass `None` for methods without parameters. This is the most flexible
    /// extension API and preserves server-specific fields exactly as
    /// `serde_json::Value`. It still applies the client's timeout, shutdown,
    /// and strict-discovery checks.
    pub async fn call_value(&self, method: &str, params: Option<Value>) -> Result<Value> {
        self.client.call_raw_value(method, params).await
    }
}
