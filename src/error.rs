use serde_json::Value;
use thiserror::Error;

use crate::capability::Feature;
use crate::transport::RequestId;

/// Result alias used by most fallible `mcsmp-rs` operations.
///
/// The default error type is [`Error`]. The optional second generic parameter
/// is provided for APIs that need to preserve another error type.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// JSON-RPC error object returned by the remote MCSMP peer.
///
/// This is the remote side's failure, as distinct from local transport,
/// serialization, capability, or timeout errors represented by [`Error`].
/// Inspect [`Self::code`], [`Self::message`], and [`Self::data`] to implement
/// server-specific recovery or diagnostics.
#[derive(Clone, Debug, PartialEq)]
pub struct RemoteError {
    /// Numeric JSON-RPC error code supplied by the server.
    ///
    /// Standard JSON-RPC codes are negative, but Minecraft or extensions may
    /// use implementation-defined values.
    pub code: i64,
    /// Human-readable message supplied by the server.
    ///
    /// This text is diagnostic output, not a stable machine-readable API.
    pub message: String,
    /// Optional implementation-specific JSON error data.
    ///
    /// The crate preserves this value without interpretation so extension
    /// clients can inspect custom server details.
    pub data: Option<Value>,
}

impl std::fmt::Display for RemoteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "JSON-RPC error {}: {}", self.code, self.message)
    }
}

impl std::error::Error for RemoteError {}

/// Failures that can occur while configuring or using an MCSMP client.
///
/// Variants distinguish local failures from remote JSON-RPC errors:
/// [`Self::Remote`] wraps a [`RemoteError`] sent by the server, while
/// configuration, transport, parsing, timeout, discovery, and compatibility
/// failures originate in the client. The enum is `non_exhaustive`; include a
/// fallback arm when matching it outside this crate.
#[derive(Clone, Debug, Error, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// A builder argument, secret, timeout, or reconnect policy was invalid.
    ///
    /// The string is intended for diagnostics and describes the local
    /// validation rule that failed.
    #[error("invalid client configuration: {0}")]
    Configuration(String),

    /// Authentication was never selected on `ClientBuilder`.
    ///
    /// Set [`crate::Auth::bearer`], [`crate::Auth::websocket_subprotocol`], or
    /// explicitly select [`crate::Auth::none`] before connecting.
    #[error(
        "authentication must be configured; use Auth::none() only for an intentionally unauthenticated endpoint"
    )]
    AuthenticationNotConfigured,

    /// The HTTP/WebSocket handshake was rejected as an authentication failure.
    ///
    /// Confirm the secret, authentication form, server enablement, and allowed
    /// origin. The contained status is the HTTP status observed during the
    /// failed WebSocket upgrade.
    #[error("management endpoint rejected authentication with HTTP {status}")]
    AuthenticationRejected {
        /// HTTP status code returned during the failed WebSocket upgrade.
        status: u16,
    },

    /// A TCP, TLS, WebSocket, or handshake operation failed locally.
    ///
    /// The string contains the underlying transport diagnostic. A reconnect
    /// policy may move the client into `Reconnecting` after such a failure.
    #[error("websocket transport error: {0}")]
    Transport(String),

    /// The client or peer closed the WebSocket before an operation completed.
    ///
    /// Outstanding requests fail rather than being replayed. Build a new
    /// client after an explicit shutdown; an automatically reconnecting client
    /// instead reports [`Self::Reconnecting`] for new calls while it recovers.
    #[error("websocket connection is closed")]
    Closed,

    /// The client is reconnecting after an unexpected transport interruption.
    ///
    /// Requests are not queued or replayed while reconnecting. Wait for
    /// [`crate::ConnectionState::Connected`] and issue a new call only when
    /// doing so is semantically safe for the operation.
    #[error("websocket client is reconnecting; requests are not queued or replayed")]
    Reconnecting,

    /// A JSON-RPC call did not receive a response before its configured deadline.
    ///
    /// The request may already have reached the server. Do not assume that a
    /// timeout means the management action did not happen.
    #[error("JSON-RPC request {id} for method `{method}` timed out")]
    Timeout {
        /// Client-generated identifier assigned to the timed-out JSON-RPC request.
        id: RequestId,
        /// Full JSON-RPC method name that did not receive a response in time.
        method: String,
    },

    /// The peer sent a WebSocket message outside the supported JSON-RPC 2.0 subset.
    ///
    /// This indicates malformed JSON, an invalid response shape, an
    /// inconsistent request identifier, or another protocol-level violation.
    #[error("JSON-RPC protocol error: {0}")]
    Protocol(String),

    /// Request parameters could not be converted to JSON before sending.
    #[error("JSON serialization failed: {0}")]
    Serialization(String),

    /// A result or notification payload could not be decoded into its Rust model.
    ///
    /// For typed API calls this often indicates a server-side protocol mismatch
    /// or an extension/version that the current model does not recognize.
    #[error("JSON deserialization failed: {0}")]
    Deserialization(String),

    /// Strict compatibility mode requires successful `rpc.discover` first.
    ///
    /// Call [`crate::Client::discover`] and retain the same connected session
    /// before issuing ordinary typed or raw methods.
    #[error("strict compatibility mode requires calling Client::discover() first")]
    DiscoveryRequired,

    /// Strict-mode discovery did not advertise a requested JSON-RPC method.
    ///
    /// The request is rejected locally and is not written to the WebSocket.
    #[error("server does not advertise JSON-RPC method `{method}`")]
    UnsupportedMethod {
        /// Full JSON-RPC method name rejected by the strict-mode preflight check.
        method: String,
    },

    /// Discovery did not imply a requested optional protocol feature.
    #[error("server does not support MCSMP feature {0:?}")]
    UnsupportedFeature(Feature),

    /// The server returned a JSON-RPC error object.
    ///
    /// Use [`RemoteError`] to inspect the remote error code, message, and
    /// optional data.
    #[error(transparent)]
    Remote(#[from] RemoteError),
}

impl Error {
    pub(crate) fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration(message.into())
    }

    pub(crate) fn protocol(message: impl Into<String>) -> Self {
        Self::Protocol(message.into())
    }

    pub(crate) fn transport(message: impl Into<String>) -> Self {
        Self::Transport(message.into())
    }
}
