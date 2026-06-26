//! Builder and immutable connection configuration.

use std::time::Duration;

use url::Url;

use crate::capability::CompatibilityMode;
use crate::transport::{open_socket, WebSocketConfig};
use crate::{Auth, Error, ReconnectPolicy, Result};

use super::Client;

const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_CHANNEL_CAPACITY: usize = 128;

/// Builder for a [`Client`].
#[derive(Clone, Debug)]
pub struct ClientBuilder {
    endpoint: Url,
    auth: Option<Auth>,
    origin: Option<String>,
    request_timeout: Duration,
    channel_capacity: usize,
    compatibility_mode: CompatibilityMode,
    reconnect_policy: ReconnectPolicy,
}

impl ClientBuilder {
    /// Creates a builder targeting an MCSMP WebSocket endpoint.
    pub fn new(endpoint: Url) -> Self {
        Self {
            endpoint,
            auth: None,
            origin: None,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            channel_capacity: DEFAULT_CHANNEL_CAPACITY,
            compatibility_mode: CompatibilityMode::default(),
            reconnect_policy: ReconnectPolicy::default(),
        }
    }

    /// Sets handshake authentication.
    pub fn auth(mut self, auth: Auth) -> Self {
        self.auth = Some(auth);
        self
    }
    /// Sets the optional HTTP `Origin` header.
    pub fn origin(mut self, origin: impl Into<String>) -> Self {
        self.origin = Some(origin.into());
        self
    }
    /// Sets the maximum time a JSON-RPC call waits for a response.
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }
    /// Sets the capacity of the local request writer queue.
    pub fn channel_capacity(mut self, capacity: usize) -> Self {
        self.channel_capacity = capacity;
        self
    }
    /// Sets the discovery-aware invocation policy.
    pub fn compatibility_mode(mut self, mode: CompatibilityMode) -> Self {
        self.compatibility_mode = mode;
        self
    }
    /// Sets the policy used after an unexpected transport interruption.
    pub fn reconnect_policy(mut self, policy: ReconnectPolicy) -> Self {
        self.reconnect_policy = policy;
        self
    }

    /// Opens the WebSocket and starts the reader and writer tasks.
    pub async fn connect(self) -> Result<Client> {
        let config = self.into_config()?;
        let socket = open_socket(&config.websocket).await?;
        Ok(Client::from_socket(socket, config))
    }

    pub(crate) fn into_config(self) -> Result<ClientConfig> {
        validate(&self)?;
        let Self {
            endpoint,
            auth,
            origin,
            request_timeout,
            channel_capacity,
            compatibility_mode,
            reconnect_policy,
        } = self;
        let auth = auth.expect("builder validation requires an authentication choice");
        Ok(ClientConfig {
            request_timeout,
            channel_capacity,
            compatibility_mode,
            reconnect_policy,
            websocket: WebSocketConfig::new(endpoint, auth, origin),
        })
    }

    #[cfg(test)]
    pub(crate) fn handshake_request(
        &self,
    ) -> Result<tokio_tungstenite::tungstenite::handshake::client::Request> {
        validate(self)?;
        let auth = self
            .auth
            .clone()
            .expect("builder validation requires an authentication choice");
        WebSocketConfig::new(self.endpoint.clone(), auth, self.origin.clone()).handshake_request()
    }
}

/// Immutable session configuration derived from a validated builder.
pub(crate) struct ClientConfig {
    pub(crate) request_timeout: Duration,
    pub(crate) channel_capacity: usize,
    pub(crate) compatibility_mode: CompatibilityMode,
    pub(crate) reconnect_policy: ReconnectPolicy,
    pub(crate) websocket: WebSocketConfig,
}

fn validate(builder: &ClientBuilder) -> Result<()> {
    match builder.endpoint.scheme() {
        "ws" | "wss" => {}
        scheme => {
            return Err(Error::configuration(format!(
                "MCSMP endpoint scheme must be ws or wss, got `{scheme}`"
            )));
        }
    }
    if builder.endpoint.host_str().is_none() {
        return Err(Error::configuration("MCSMP endpoint must include a host"));
    }
    if builder.auth.is_none() {
        return Err(Error::AuthenticationNotConfigured);
    }
    if builder.request_timeout.is_zero() {
        return Err(Error::configuration(
            "request timeout must be greater than zero",
        ));
    }
    if builder.channel_capacity == 0 {
        return Err(Error::configuration(
            "channel capacity must be greater than zero",
        ));
    }
    builder
        .reconnect_policy
        .validate()
        .map_err(Error::configuration)
}

#[cfg(test)]
mod tests {
    use tokio_tungstenite::tungstenite::http::header::{
        AUTHORIZATION, ORIGIN, SEC_WEBSOCKET_PROTOCOL,
    };
    use url::Url;

    use super::*;

    #[test]
    fn bearer_handshake_contains_authorization_and_origin() {
        let builder = Client::builder(Url::parse("wss://localhost:25585").unwrap())
            .auth(Auth::bearer("secret").unwrap())
            .origin("mcsmp-rs-test");
        let request = builder.handshake_request().unwrap();
        assert_eq!(request.headers()[AUTHORIZATION], "Bearer secret");
        assert_eq!(request.headers()[ORIGIN], "mcsmp-rs-test");
    }

    #[test]
    fn subprotocol_handshake_contains_browser_compatible_credentials() {
        let builder = Client::builder(Url::parse("ws://localhost:25585").unwrap())
            .auth(Auth::websocket_subprotocol("secret").unwrap());
        let request = builder.handshake_request().unwrap();
        assert_eq!(
            request.headers()[SEC_WEBSOCKET_PROTOCOL],
            "minecraft-v1,secret"
        );
    }

    #[test]
    fn builder_rejects_invalid_reconnect_policy() {
        let builder = Client::builder(Url::parse("ws://localhost:25585").unwrap())
            .auth(Auth::none())
            .reconnect_policy(ReconnectPolicy::fixed(Duration::ZERO, Some(1)));
        assert!(matches!(validate(&builder), Err(Error::Configuration(_))));
    }
}
