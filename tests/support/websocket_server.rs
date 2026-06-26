use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

pub async fn bind() -> (Url, TcpListener) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test server");
    let address = listener.local_addr().expect("read test server address");
    (
        Url::parse(&format!("ws://{address}")).expect("valid test URL"),
        listener,
    )
}

pub async fn receive_request<S>(socket: &mut tokio_tungstenite::WebSocketStream<S>) -> Value
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    loop {
        match socket
            .next()
            .await
            .expect("frame expected")
            .expect("valid frame")
        {
            Message::Text(text) => {
                return serde_json::from_str(text.as_ref()).expect("JSON-RPC request");
            }
            Message::Ping(payload) => socket.send(Message::Pong(payload)).await.expect("pong"),
            Message::Close(_) => panic!("client closed before sending expected request"),
            _ => {}
        }
    }
}

pub async fn send_result<S>(
    socket: &mut tokio_tungstenite::WebSocketStream<S>,
    request: &Value,
    result: Value,
) where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    socket
        .send(Message::Text(
            json!({"jsonrpc":"2.0", "id": request["id"], "result": result})
                .to_string()
                .into(),
        ))
        .await
        .expect("send result");
}
