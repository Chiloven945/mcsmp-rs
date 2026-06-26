//! Public client connection lifecycle state.

/// The current lifecycle state of a client connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConnectionState {
    /// WebSocket handshake succeeded and requests can be sent.
    Connected,
    /// The client is waiting before or attempting an automatic reconnection.
    ///
    /// Calls made in this state return [`crate::Error::Reconnecting`].
    Reconnecting,
    /// Shutdown was requested and connection tasks are draining.
    Closing,
    /// The connection shut down cleanly or the peer closed it.
    Closed,
    /// A transport or protocol failure terminated the connection permanently.
    Failed,
}
