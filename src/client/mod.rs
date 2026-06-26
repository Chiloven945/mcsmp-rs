//! Public MCSMP client facade.

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
use crate::transport::{start_session, SessionConfig, SessionController, Socket};
use crate::{ReconnectPolicy, Result};

mod config;
mod state;

pub use config::ClientBuilder;
pub use state::ConnectionState;

/// An asynchronous, cloneable MCSMP WebSocket client.
#[derive(Clone, Debug)]
pub struct Client {
    inner: Arc<SessionController>,
}

impl Client {
    /// Starts building a client for `endpoint`.
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

    /// Returns the current connection lifecycle state.
    pub fn state(&self) -> ConnectionState {
        self.inner.state()
    }

    /// Subscribes to strongly typed MCSMP notifications.
    pub fn subscribe(&self) -> EventStream {
        EventStream::new(
            self.inner.subscribe_events(),
            self.inner.subscribe_event_shutdown(),
        )
    }

    /// Subscribes to normalized raw JSON-RPC notifications.
    pub fn subscribe_notifications(&self) -> broadcast::Receiver<RawNotification> {
        self.inner.subscribe_raw_notifications()
    }

    /// Returns the untyped JSON-RPC extension API.
    pub fn raw(&self) -> RawApi {
        RawApi::new(self.clone())
    }
    /// Returns allowlist operations.
    pub fn allowlist(&self) -> AllowlistApi {
        AllowlistApi::new(self.clone())
    }
    /// Returns user-ban operations.
    pub fn bans(&self) -> BansApi {
        BansApi::new(self.clone())
    }
    /// Returns IP-ban operations.
    pub fn ip_bans(&self) -> IpBansApi {
        IpBansApi::new(self.clone())
    }
    /// Returns connected-player operations.
    pub fn players(&self) -> PlayersApi {
        PlayersApi::new(self.clone())
    }
    /// Returns operator-list operations.
    pub fn operators(&self) -> OperatorsApi {
        OperatorsApi::new(self.clone())
    }
    /// Returns server lifecycle and messaging operations.
    pub fn server(&self) -> ServerApi {
        ServerApi::new(self.clone())
    }
    /// Returns live server-settings operations.
    pub fn server_settings(&self) -> ServerSettingsApi {
        ServerSettingsApi::new(self.clone())
    }
    /// Returns gamerule operations.
    pub fn gamerules(&self) -> GamerulesApi {
        GamerulesApi::new(self.clone())
    }

    /// Returns the configured discovery-aware invocation policy.
    pub fn compatibility_mode(&self) -> CompatibilityMode {
        self.inner.compatibility_mode()
    }
    /// Returns the configured automatic reconnection policy.
    pub fn reconnect_policy(&self) -> &ReconnectPolicy {
        self.inner.reconnect_policy()
    }
    /// Returns the latest cached capability snapshot.
    pub fn capabilities(&self) -> Option<Capabilities> {
        self.inner.capabilities()
    }

    /// Calls `rpc.discover`, caches, and returns the server capability snapshot.
    pub async fn discover(&self) -> Result<Capabilities> {
        let capabilities = capability::discover_capabilities(self).await?;
        self.inner.replace_capabilities(capabilities.clone());
        Ok(capabilities)
    }

    /// Closes the socket, stops background tasks, and fails outstanding calls.
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
