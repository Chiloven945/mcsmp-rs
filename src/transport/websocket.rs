//! WebSocket handshake construction and connection establishment.
//!
//! This module intentionally owns the HTTP/WebSocket boundary. Keeping it
//! separate from `client` makes the public client API independent from the
//! details that are also needed by the reconnect runtime.

use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Error as WebSocketError;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::http::header::{AUTHORIZATION, ORIGIN, SEC_WEBSOCKET_PROTOCOL};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use url::Url;

use crate::{Auth, Error, Result};

/// The concrete websocket stream used by the Tokio runtime.
pub(crate) type Socket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// Immutable handshake details retained for the initial connection and reconnect attempts.
#[derive(Clone, Debug)]
pub(crate) struct WebSocketConfig {
    endpoint: Url,
    auth: Auth,
    origin: Option<String>,
}

impl WebSocketConfig {
    pub(crate) fn new(endpoint: Url, auth: Auth, origin: Option<String>) -> Self {
        Self {
            endpoint,
            auth,
            origin,
        }
    }

    pub(crate) fn handshake_request(
        &self,
    ) -> Result<tokio_tungstenite::tungstenite::handshake::client::Request> {
        let mut request = self
            .endpoint
            .as_str()
            .into_client_request()
            .map_err(|error| {
                Error::configuration(format!("invalid websocket endpoint: {error}"))
            })?;

        match &self.auth {
            Auth::Bearer(secret) => set_header(
                request.headers_mut(),
                AUTHORIZATION,
                &format!("Bearer {}", secret.expose()),
            )?,
            Auth::WebSocketSubprotocol(secret) => set_header(
                request.headers_mut(),
                SEC_WEBSOCKET_PROTOCOL,
                &format!("minecraft-v1,{}", secret.expose()),
            )?,
            Auth::None => {}
        }

        if let Some(origin) = &self.origin {
            set_header(request.headers_mut(), ORIGIN, origin)?;
        }

        Ok(request)
    }
}

/// Opens one authenticated WebSocket session.
pub(crate) async fn open_socket(config: &WebSocketConfig) -> Result<Socket> {
    let request = config.handshake_request()?;
    let (socket, _) = connect_async(request).await.map_err(map_connect_error)?;
    Ok(socket)
}

fn set_header(
    headers: &mut tokio_tungstenite::tungstenite::http::HeaderMap,
    name: tokio_tungstenite::tungstenite::http::header::HeaderName,
    value: &str,
) -> Result<()> {
    let value = value
        .parse::<HeaderValue>()
        .map_err(|error| Error::configuration(format!("invalid `{name}` header value: {error}")))?;
    headers.insert(name, value);
    Ok(())
}

fn map_connect_error(error: WebSocketError) -> Error {
    match error {
        WebSocketError::Http(response) if response.status().as_u16() == 401 => {
            Error::AuthenticationRejected { status: 401 }
        }
        error => Error::transport(error.to_string()),
    }
}
