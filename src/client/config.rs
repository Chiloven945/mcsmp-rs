//! Fluent client construction and validated connection configuration.
//!
//! `ClientBuilder` collects all user-configurable connection options before a
//! WebSocket is opened. Validation happens in [`ClientBuilder::connect`], so
//! a builder can be freely composed and cloned without performing I/O.

use std::time::Duration;

use url::Url;

use crate::capability::CompatibilityMode;
use crate::transport::{WebSocketConfig, open_socket};
use crate::{Auth, Error, ReconnectPolicy, Result};

use super::Client;

const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_CHANNEL_CAPACITY: usize = 128;

/// Fluent builder for a [`Client`] connection.
///
/// Defaults are intentionally conservative:
///
/// - a ten-second JSON-RPC response timeout;
/// - a request writer queue capacity of 128;
/// - [`crate::CompatibilityMode::Compatible`]; and
/// - [`crate::ReconnectPolicy::Never`].
///
/// Authentication has no implicit default. Calling [`Self::connect`] without
/// [`Self::auth`] returns [`crate::Error::AuthenticationNotConfigured`], which
/// prevents accidentally sending management traffic without a deliberate
/// credential decision.
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
    ///
    /// This function does not validate or connect immediately. Validation is
    /// deferred until [`Self::connect`]. The endpoint must ultimately use
    /// `ws://` or `wss://` and contain a host; a normal deployment should use
    /// `wss://` so that the management secret and JSON-RPC payloads are
    /// protected in transit.
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

    /// Sets the authentication form used during the WebSocket handshake.
    ///
    /// Native applications should normally choose [`crate::Auth::bearer`],
    /// which emits `Authorization: Bearer <secret>`. The subprotocol form is
    /// useful for browser-originated connections. Passing
    /// [`crate::Auth::none`] is explicit and only appropriate for deliberately
    /// unauthenticated endpoints.
    pub fn auth(mut self, auth: Auth) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Sets the HTTP `Origin` header sent during the WebSocket handshake.
    ///
    /// Minecraft can reject connections whose origin is absent from
    /// `management-server-allowed-origins`. This value is sent as provided and
    /// is not validated as a URL, matching the server property's semantics.
    /// Calling this method more than once replaces the previous value.
    pub fn origin(mut self, origin: impl Into<String>) -> Self {
        self.origin = Some(origin.into());
        self
    }

    /// Sets the maximum time an individual JSON-RPC call waits for a response.
    ///
    /// The timeout applies after a request enters the client's writer queue.
    /// A zero duration is rejected by [`Self::connect`]. Timing out removes the
    /// request's local response waiter; a late server response is ignored and
    /// does not close the connection.
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// Sets the capacity of the local outbound request queue.
    ///
    /// The queue absorbs short bursts of concurrent calls before the single
    /// WebSocket writer sends them. A zero capacity is rejected by
    /// [`Self::connect`]. This is a local back-pressure limit, not a server
    /// concurrency limit.
    pub fn channel_capacity(mut self, capacity: usize) -> Self {
        self.channel_capacity = capacity;
        self
    }

    /// Sets the discovery-aware invocation policy.
    ///
    /// The default [`crate::CompatibilityMode::Compatible`] accepts supported
    /// historical wire forms. [`crate::CompatibilityMode::Strict`] requires a
    /// successful `Client::discover` before ordinary calls and locally rejects
    /// methods the server did not advertise. [`crate::CompatibilityMode::Permissive`]
    /// is useful for experimentation with extension methods.
    pub fn compatibility_mode(mut self, mode: CompatibilityMode) -> Self {
        self.compatibility_mode = mode;
        self
    }

    /// Sets automatic reconnection behavior after an unexpected interruption.
    ///
    /// Reconnection never replays pending or already-sent JSON-RPC requests.
    /// A caller must decide whether a failed management action is safe to issue
    /// again after the client returns to `Connected`. Invalid delays and attempt
    /// limits are rejected when [`Self::connect`] validates the builder.
    pub fn reconnect_policy(mut self, policy: ReconnectPolicy) -> Self {
        self.reconnect_policy = policy;
        self
    }

    /// Validates the builder, opens the WebSocket, and starts session tasks.
    ///
    /// The Minecraft server must enable its management endpoint before this
    /// call. A typical `server.properties` configuration is:
    ///
    /// ```text
    /// management-server-enabled=true
    /// management-server-host=127.0.0.1
    /// management-server-port=25585
    /// management-server-secret=<secret>
    /// management-server-allowed-origins=mcsmp-rs
    /// management-server-tls-enabled=true
    /// ```
    ///
    /// Use a `wss://` endpoint when TLS is enabled. The `origin` value, when
    /// configured, must be present in `management-server-allowed-origins`.
    /// `Auth::bearer` sends the configured management secret in the standard
    /// HTTP authorization header.
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
