//! Public client connection lifecycle state.
//!
//! [`ConnectionState`] is a snapshot returned by [`crate::Client::state`]. It
//! does not perform network I/O and can change immediately after it is read;
//! callers should still handle errors returned by an actual request.

/// Current lifecycle state of an MCSMP client connection.
///
/// The enum is `non_exhaustive`; downstream code should include a fallback arm
/// to remain source-compatible with future states. A state value is advisory:
/// a peer can close a socket between observing `Connected` and submitting a
/// request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConnectionState {
    /// WebSocket handshake succeeded and new requests can be submitted.
    Connected,
    /// The client is waiting before or attempting an automatic reconnection.
    ///
    /// Calls made in this state return [`crate::Error::Reconnecting`].
    Reconnecting,
    /// Explicit shutdown was requested and connection tasks are draining.
    ///
    /// New calls do not begin during this state.
    Closing,
    /// The session shut down cleanly or the peer closed it without reconnecting.
    ///
    /// This is terminal for the current client instance.
    Closed,
    /// A transport or protocol failure terminated the session permanently.
    ///
    /// This is terminal for the current client instance. Build a new client to
    /// establish another connection.
    Failed,
}
