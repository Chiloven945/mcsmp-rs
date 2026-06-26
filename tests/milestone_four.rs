use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use mcsmp_rs::{
    Auth, Client, CompatibilityMode, ConnectionState, Error, Event, ReconnectPolicy,
};
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
async fn compatible_mode_normalizes_legacy_notifications_and_emits_typed_events() {
    let (url, listener) = bind_server().await;
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        let request = receive_request(&mut socket).await;
        assert_eq!(request["method"], "minecraft:server/status");
        socket
            .send(Message::Text(
                json!({
                    "jsonrpc": "2.0",
                    "method": "notification:players/joined",
                    "params": {"player": {"name": "Alex"}}
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();
        send_result(
            &mut socket,
            &request,
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

    let client = Client::builder(url).auth(Auth::none()).connect().await.unwrap();
    let mut events = client.subscribe();
    let mut raw = client.subscribe_notifications();

    let _ = client.server().status().await.unwrap();
    assert!(matches!(
        events.recv().await.unwrap(),
        Event::PlayerJoined { player } if player.name() == Some("Alex")
    ));
    assert_eq!(
        raw.recv().await.unwrap().method,
        "minecraft:notification/players/joined"
    );

    client.shutdown().await.unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn preview_world_notifications_are_typed_only_after_discovery() {
    let (url, listener) = bind_server().await;
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();

        let discover = receive_request(&mut socket).await;
        assert_eq!(discover["method"], "rpc.discover");
        send_result(
            &mut socket,
            &discover,
            json!({
                "protocolVersion": "3.1.0",
                "methods": ["rpc.discover", "minecraft:server/status"],
                "notifications": ["minecraft:notification/world/upgrade_progress"]
            }),
        )
        .await;

        let status = receive_request(&mut socket).await;
        assert_eq!(status["method"], "minecraft:server/status");
        socket
            .send(Message::Text(
                json!({
                    "jsonrpc": "2.0",
                    "method": "minecraft:notification/world/upgrade_progress",
                    "params": {"progress": 0.25}
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();
        send_result(
            &mut socket,
            &status,
            json!({
                "status": {
                    "started": false,
                    "players": [],
                    "version": {"name": "26.3", "protocol": 1}
                }
            }),
        )
        .await;
    });

    let client = Client::builder(url).auth(Auth::none()).connect().await.unwrap();
    let mut events = client.subscribe();
    client.discover().await.unwrap();
    let _ = client.server().status().await.unwrap();

    assert!(matches!(
        events.recv().await.unwrap(),
        Event::WorldUpgradeProgress { progress } if (progress - 0.25).abs() < f64::EPSILON
    ));

    client.shutdown().await.unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn strict_mode_rejects_legacy_notification_prefixes() {
    let (url, listener) = bind_server().await;
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        socket
            .send(Message::Text(
                json!({
                    "jsonrpc": "2.0",
                    "method": "notification:server/started"
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();
    });

    let client = Client::builder(url)
        .auth(Auth::none())
        .compatibility_mode(CompatibilityMode::Strict)
        .connect()
        .await
        .unwrap();

    timeout(Duration::from_secs(1), async {
        while client.state() == ConnectionState::Connected {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(client.state(), ConnectionState::Failed);

    client.shutdown().await.unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn reconnect_does_not_replay_inflight_requests_and_refreshes_discovery() {
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
