use serde_json::Value;
use thiserror::Error;

use crate::RequestId;

/// The result type used by this crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error returned by the remote JSON-RPC peer.
#[derive(Clone, Debug, PartialEq)]
pub struct RemoteError {
    /// JSON-RPC error code supplied by the server.
    pub code: i64,
    /// Human-readable server error message.
    pub message: String,
    /// Optional implementation-specific error data.
    pub data: Option<Value>,
}

impl std::fmt::Display for RemoteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "JSON-RPC error {}: {}", self.code, self.message)
    }
}

impl std::error::Error for RemoteError {}

/// Failures produced locally while creating or using an MCSMP client.
#[derive(Clone, Debug, Error, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// A builder argument or authentication value was invalid.
    #[error("invalid client configuration: {0}")]
    Configuration(String),

    /// Authentication was omitted without explicitly selecting `Auth::None`.
    #[error(
        "authentication must be configured; use Auth::none() only for an intentionally unauthenticated endpoint"
    )]
    AuthenticationNotConfigured,

    /// The remote endpoint rejected the handshake with an authentication error.
    #[error("management endpoint rejected authentication with HTTP {status}")]
    AuthenticationRejected {
        /// HTTP status code returned by the server.
        status: u16,
    },

    /// A connection or WebSocket transport operation failed.
    #[error("websocket transport error: {0}")]
    Transport(String),

    /// The peer closed the WebSocket before an operation completed.
    #[error("websocket connection is closed")]
    Closed,

    /// A JSON-RPC call exceeded its configured deadline.
    #[error("JSON-RPC request {id} for method `{method}` timed out")]
    Timeout {
        /// Client-generated request identifier.
        id: RequestId,
        /// JSON-RPC method that did not receive a response in time.
        method: String,
    },

    /// The peer sent a message that does not conform to the supported JSON-RPC
    /// 2.0 subset.
    #[error("JSON-RPC protocol error: {0}")]
    Protocol(String),

    /// JSON serialization failed before a request could be sent.
    #[error("JSON serialization failed: {0}")]
    Serialization(String),

    /// JSON deserialization failed while decoding a response into a caller type.
    #[error("JSON deserialization failed: {0}")]
    Deserialization(String),

    /// The server returned a JSON-RPC error object.
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
