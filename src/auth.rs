use std::fmt;

use crate::Error;

/// A management-server secret that is safe to include in a request header.
///
/// The Minecraft server normally generates a 40-character alphanumeric secret.
/// This type intentionally validates only header safety rather than the exact
/// server-side format, so callers can use development servers with alternate
/// credential provisioning.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    /// Creates a secret after validating that it is non-empty and cannot inject
    /// an additional HTTP header.
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

/// Authentication to apply during the WebSocket handshake.
#[derive(Clone, Debug)]
pub enum Auth {
    /// Sends the management secret in `Authorization: Bearer <secret>`.
    Bearer(Secret),
    /// Sends `minecraft-v1,<secret>` in `Sec-WebSocket-Protocol`.
    ///
    /// This form exists primarily for browser-compatible clients. The
    /// `minecraft-v1` token is a WebSocket subprotocol convention, not the
    /// MCSMP semantic protocol version.
    WebSocketSubprotocol(Secret),
    /// Sends no management credentials. This must be selected deliberately.
    None,
}

impl Auth {
    /// Builds bearer-token authentication.
    pub fn bearer(secret: impl Into<String>) -> Result<Self, Error> {
        Ok(Self::Bearer(Secret::new(secret)?))
    }

    /// Builds WebSocket-subprotocol authentication.
    pub fn websocket_subprotocol(secret: impl Into<String>) -> Result<Self, Error> {
        Ok(Self::WebSocketSubprotocol(Secret::new(secret)?))
    }

    /// Selects an unauthenticated handshake explicitly.
    pub const fn none() -> Self {
        Self::None
    }
}
