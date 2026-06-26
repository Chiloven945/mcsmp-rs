//! Public MCSMP client facade.
//!
//! This module contains the cloneable [`Client`] connection facade, the
//! [`ClientBuilder`] used to configure it, and [`ConnectionState`] values used
//! to observe session lifecycle. Most applications construct a client with
//! `Client::builder(...).auth(...).connect().await` and then obtain typed
//! resource handles from the resulting client.

use std::sync::Arc;

use serde_json::Value;
use tokio::sync::broadcast;
use url::Url;

use crate::api::{
    AllowlistApi, BansApi, GamerulesApi, IpBansApi, OperatorsApi, PlayersApi, RawApi, ServerApi,
    ServerSettingsApi,
};
use crate::capability::{self, Capabilities, CompatibilityMode};
use crate::events::{EventStream, RawNotification};
use crate::transport::{SessionConfig, SessionController, Socket, start_session};
use crate::{ReconnectPolicy, Result};

mod config;
mod state;

pub use config::ClientBuilder;
pub use state::ConnectionState;

/// A cloneable asynchronous MCSMP client backed by one WebSocket session.
///
/// A `Client` owns shared session state through an `Arc`; cloning it creates
/// another lightweight handle to the same connection rather than another
/// socket. Calls made through clones are multiplexed using JSON-RPC request
/// identifiers and may complete out of order.
///
/// The client starts reader and writer tasks when [`ClientBuilder::connect`]
/// succeeds. Use [`Self::shutdown`] to close those tasks explicitly. If the
/// peer disconnects unexpectedly, behavior is controlled by
/// [`crate::ReconnectPolicy`]; requests that were pending at the time of the
/// disconnect are never replayed.
#[derive(Clone, Debug)]
pub struct Client {
    inner: Arc<SessionController>,
}

impl Client {
    /// Starts configuring a client for an MCSMP WebSocket endpoint.
    ///
    /// `endpoint` must use either the `ws` or `wss` scheme and include a host.
    /// Choose `wss` for normal deployments, because Minecraft enables TLS by
    /// default. The builder requires an explicit authentication choice before
    /// connecting; use [`crate::Auth::bearer`] for native clients,
    /// [`crate::Auth::websocket_subprotocol`] for browser-compatible
    /// handshakes, or [`crate::Auth::none`] only when the endpoint truly does
    /// not require a secret.
    pub fn builder(endpoint: Url) -> ClientBuilder {
        ClientBuilder::new(endpoint)
    }

    pub(crate) fn from_socket(socket: Socket, config: config::ClientConfig) -> Self {
        let session_config = SessionConfig {
            request_timeout: config.request_timeout,
            channel_capacity: config.channel_capacity,
            compatibility_mode: config.compatibility_mode,
            reconnect_policy: config.reconnect_policy,
            websocket: config.websocket,
        };
        let inner = Arc::new(SessionController::new(session_config));
        start_session(&inner, socket);
        Self { inner }
    }

    pub(crate) fn from_controller(inner: Arc<SessionController>) -> Self {
        Self { inner }
    }

    /// Returns the current lifecycle state without performing network I/O.
    ///
    /// `Connected` means new requests may be submitted. `Reconnecting` means
    /// an automatic reconnect policy is active and new requests return
    /// [`crate::Error::Reconnecting`]. `Closed` and `Failed` are terminal for
    /// the current client instance.
    pub fn state(&self) -> ConnectionState {
        self.inner.state()
    }

    /// Creates an independent stream of strongly typed MCSMP notifications.
    ///
    /// Each call creates a new subscription to the client's bounded broadcast
    /// channel. A slow subscriber can miss events and then receives
    /// [`crate::EventStreamError::Lagged`]; query the relevant resource API to
    /// re-synchronize authoritative state. Unknown, extension-defined, and
    /// malformed notifications are represented as [`crate::Event::Unknown`].
    ///
    /// The stream is tied to the current client lifecycle. It drains events
    /// already queued before a terminal close, then ends with
    /// [`crate::EventStreamError::Closed`].
    pub fn subscribe(&self) -> EventStream {
        EventStream::new(
            self.inner.subscribe_events(),
            self.inner.subscribe_event_shutdown(),
        )
    }

    /// Creates an independent receiver for normalized raw notifications.
    ///
    /// Use this when an application needs extension-specific payloads that do
    /// not have a typed [`crate::Event`] variant. Historical notification names
    /// are normalized according to the configured
    /// [`crate::CompatibilityMode`]. The returned Tokio broadcast receiver has
    /// the same bounded-buffer and lag behavior as `subscribe`.
    pub fn subscribe_notifications(&self) -> broadcast::Receiver<RawNotification> {
        self.inner.subscribe_raw_notifications()
    }

    /// Returns a generic JSON-RPC handle for extension-defined methods.
    ///
    /// The returned [`crate::RawApi`] shares this client's connection,
    /// discovery policy, timeout, and reconnect behavior.
    pub fn raw(&self) -> RawApi {
        RawApi::new(self.clone())
    }
    /// Returns the typed `minecraft:allowlist` resource handle.
    ///
    /// Creating a handle is synchronous and inexpensive; no network request is
    /// made until a method such as `list` or `add` is awaited.
    pub fn allowlist(&self) -> AllowlistApi {
        AllowlistApi::new(self.clone())
    }
    /// Returns the typed `minecraft:bans` resource handle.
    pub fn bans(&self) -> BansApi {
        BansApi::new(self.clone())
    }
    /// Returns the typed `minecraft:ip_bans` resource handle.
    pub fn ip_bans(&self) -> IpBansApi {
        IpBansApi::new(self.clone())
    }
    /// Returns the typed `minecraft:players` resource handle.
    pub fn players(&self) -> PlayersApi {
        PlayersApi::new(self.clone())
    }
    /// Returns the typed `minecraft:operators` resource handle.
    pub fn operators(&self) -> OperatorsApi {
        OperatorsApi::new(self.clone())
    }
    /// Returns the typed `minecraft:server` resource handle.
    pub fn server(&self) -> ServerApi {
        ServerApi::new(self.clone())
    }
    /// Returns the typed `minecraft:serversettings` resource handle.
    pub fn server_settings(&self) -> ServerSettingsApi {
        ServerSettingsApi::new(self.clone())
    }
    /// Returns the typed `minecraft:gamerules` resource handle.
    pub fn gamerules(&self) -> GamerulesApi {
        GamerulesApi::new(self.clone())
    }

    /// Returns this client's discovery-aware invocation policy.
    ///
    /// The policy is fixed when the client is built. See
    /// [`crate::CompatibilityMode`] for the difference between strict,
    /// compatible, and permissive behavior.
    pub fn compatibility_mode(&self) -> CompatibilityMode {
        self.inner.compatibility_mode()
    }
    /// Returns the configured automatic reconnection policy.
    ///
    /// This returns configuration only; it does not indicate whether the
    /// client is currently reconnecting. Inspect [`Self::state`] for current
    /// lifecycle state.
    pub fn reconnect_policy(&self) -> &ReconnectPolicy {
        self.inner.reconnect_policy()
    }
    /// Returns the latest capability snapshot cached by discovery.
    ///
    /// This is `None` until [`Self::discover`] succeeds. The cache is cleared
    /// when a new session is established after reconnecting because another
    /// endpoint or server state may advertise different capabilities.
    pub fn capabilities(&self) -> Option<Capabilities> {
        self.inner.capabilities()
    }

    /// Calls `rpc.discover`, caches the result, and returns a capability snapshot.
    ///
    /// Discovery is required before normal calls in
    /// [`crate::CompatibilityMode::Strict`]. In other modes it is still useful
    /// for feature gating, inspecting extension methods, and determining
    /// whether preview notifications such as world-upgrade events are safe to
    /// treat as typed events.
    ///
    /// The snapshot preserves the unmodified discovery payload in
    /// [`crate::Capabilities::raw_schema`] in addition to extracting methods,
    /// notifications, version, and inferred features.
    pub async fn discover(&self) -> Result<Capabilities> {
        let capabilities = capability::discover_capabilities(self).await?;
        self.inner.replace_capabilities(capabilities.clone());
        Ok(capabilities)
    }

    /// Explicitly closes the WebSocket session and stops background tasks.
    ///
    /// Outstanding calls complete with [`crate::Error::Closed`]. The operation
    /// is intended to be called during orderly application shutdown and is
    /// idempotent from the caller's perspective: after shutdown, this client
    /// cannot be reused or reconnected. Build a new client to establish a new
    /// session.
    pub async fn shutdown(&self) -> Result<()> {
        self.inner.shutdown().await
    }

    pub(crate) async fn call_discovery_value(&self) -> Result<Value> {
        self.inner.call_value("rpc.discover", None).await
    }

    pub(crate) async fn call_typed_value(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value> {
        self.inner.ensure_method_allowed(method)?;
        self.inner.call_value(method, params).await
    }

    pub(crate) async fn call_raw_value(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value> {
        if method != "rpc.discover" {
            self.inner.ensure_method_allowed(method)?;
        }
        self.inner.call_value(method, params).await
    }
}
