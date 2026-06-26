use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use mcsmp_rs::{Auth, Client, ConnectionState, Error, ReconnectPolicy};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio::time::{sleep, timeout};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

async fn bind_server() -> (Url, TcpListener) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    (Url::parse(&format!("ws://{address}")).unwrap(), listener)
}

async fn receive_request<S>(socket: &mut tokio_tungstenite::WebSocketStream<S>) -> Value
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    loop {
        match socket.next().await.unwrap().unwrap() {
            Message::Text(text) => return serde_json::from_str(text.as_ref()).unwrap(),
            Message::Ping(payload) => socket.send(Message::Pong(payload)).await.unwrap(),
            Message::Close(_) => panic!("client closed before sending expected request"),
            _ => {}
        }
    }
}

async fn send_result<S>(
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
        .unwrap();
}

#[tokio::test]
async fn reconnect_fails_pending_calls_without_replaying_them() {
    let (url, listener) = bind_server().await;
    let server = tokio::spawn(async move {
        let (first_stream, _) = listener.accept().await.unwrap();
        let mut first = accept_async(first_stream).await.unwrap();
        let inflight = receive_request(&mut first).await;
        assert_eq!(inflight["method"], "minecraft:players");
        first.send(Message::Close(None)).await.unwrap();

        let (second_stream, _) = listener.accept().await.unwrap();
        let mut second = accept_async(second_stream).await.unwrap();
        let discover = receive_request(&mut second).await;
        assert_eq!(discover["method"], "rpc.discover");
        send_result(
            &mut second,
            &discover,
            json!({
                "protocolVersion": "3.0.0",
                "methods": ["rpc.discover", "minecraft:server/status"],
                "notifications": []
            }),
        )
        .await;

        let status = receive_request(&mut second).await;
        assert_eq!(status["method"], "minecraft:server/status");
        send_result(
            &mut second,
            &status,
            json!({
                "status": {
                    "started": true,
                    "players": [],
                    "version": {"name": "26.2", "protocol": 1}
                }
            }),
        )
        .await;
    });

    let client = Client::builder(url)
        .auth(Auth::none())
        .reconnect_policy(ReconnectPolicy::fixed(Duration::from_millis(5), Some(5)))
        .connect()
        .await
        .unwrap();

    assert!(matches!(client.players().list().await, Err(Error::Closed)));

    timeout(Duration::from_secs(1), async {
        loop {
            if client.state() == ConnectionState::Connected && client.capabilities().is_some() {
                break;
            }
            sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();

    assert!(client.server().status().await.unwrap().started);
    client.shutdown().await.unwrap();
    server.await.unwrap();
}
