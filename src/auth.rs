use std::fmt;

use crate::Error;

/// Validated management-server secret suitable for an HTTP handshake header.
///
/// Minecraft normally generates a forty-character alphanumeric secret in
/// `server.properties`. This type deliberately validates header safety rather
/// than duplicating every server-side format rule: it rejects empty strings and
/// CR/LF characters, but accepts alternate development or proxy credential
/// formats.
///
/// `Debug` output is redacted, preventing accidental credential disclosure in
/// normal logs. The secret is still held in ordinary process memory; callers
/// remain responsible for avoiding logs, telemetry, and source-control
/// exposure of the original value.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    /// Creates a secret after applying local HTTP-header safety checks.
    ///
    /// Empty input and values containing carriage return or line feed are
    /// rejected with [`crate::Error::Configuration`]. The function does not
    /// verify the server's usual forty-character alphanumeric requirement,
    /// because compatible development servers and proxies may use another
    /// provisioning format.
    pub fn new(value: impl Into<String>) -> Result<Self, Error> {
        let value = value.into();
        if value.is_empty() {
            return Err(Error::configuration("management secret must not be empty"));
        }
        if value.contains(['\r', '\n']) {
            return Err(Error::configuration(
                "management secret must not contain carriage returns or line feeds",
            ));
        }
        Ok(Self(value))
    }

    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Secret([REDACTED])")
    }
}

/// Authentication configuration applied during the WebSocket handshake.
///
/// The management server supports a native HTTP bearer form and a
/// browser-oriented WebSocket subprotocol form. Choose the form appropriate to
/// the caller; this enum does not perform authentication until
/// [`crate::ClientBuilder::connect`] opens the socket.
#[derive(Clone, Debug)]
pub enum Auth {
    /// Sends the secret as `Authorization: Bearer <secret>`.
    ///
    /// This is the recommended form for native Rust applications and command
    /// line tools. The secret remains redacted in `Debug` output.
    Bearer(Secret),
    /// Sends `minecraft-v1,<secret>` in `Sec-WebSocket-Protocol`.
    ///
    /// This form exists primarily for browser-compatible clients because a
    /// browser WebSocket constructor can populate the subprotocol header but
    /// cannot freely set `Authorization`. `minecraft-v1` is a handshake
    /// convention, not the MCSMP semantic protocol version described by
    /// [`crate::ProtocolVersion`].
    WebSocketSubprotocol(Secret),
    /// Sends no management credentials.
    ///
    /// This variant must be selected deliberately through [`Self::none`].
    /// Most Minecraft management endpoints reject it with an HTTP
    /// authentication failure.
    None,
}

impl Auth {
    /// Creates native bearer-token authentication from a secret string.
    ///
    /// Returns [`crate::Error::Configuration`] when the value is empty or
    /// unsafe for an HTTP header.
    pub fn bearer(secret: impl Into<String>) -> Result<Self, Error> {
        Ok(Self::Bearer(Secret::new(secret)?))
    }

    /// Creates browser-compatible WebSocket-subprotocol authentication.
    ///
    /// Returns [`crate::Error::Configuration`] when the value is empty or
    /// unsafe for an HTTP header.
    pub fn websocket_subprotocol(secret: impl Into<String>) -> Result<Self, Error> {
        Ok(Self::WebSocketSubprotocol(Secret::new(secret)?))
    }

    /// Explicitly selects an unauthenticated handshake.
    ///
    /// This is not the default because missing authentication is usually an
    /// operational mistake. Use it only for a trusted endpoint configured to
    /// accept unauthenticated management connections.
    pub const fn none() -> Self {
        Self::None
    }
}
