mod support;

use std::time::Duration;

use futures_util::SinkExt;
use mcsmp_rs::{Auth, Client, CompatibilityMode, ConnectionState, Event};
use serde_json::json;
use tokio::time::timeout;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn events_normalize_legacy_names_and_decode_payloads() {
    let (url, listener) = support::websocket_server::bind().await;
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        let request = support::websocket_server::receive_request(&mut socket).await;
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
        support::websocket_server::send_result(
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

    let client = Client::builder(url)
        .auth(Auth::none())
        .connect()
        .await
        .unwrap();
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
    let (url, listener) = support::websocket_server::bind().await;
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();

        let discover = support::websocket_server::receive_request(&mut socket).await;
        assert_eq!(discover["method"], "rpc.discover");
        support::websocket_server::send_result(
            &mut socket,
            &discover,
            json!({
                "protocolVersion": "3.1.0",
                "methods": ["rpc.discover", "minecraft:server/status"],
                "notifications": ["minecraft:notification/world/upgrade_progress"]
            }),
        )
        .await;

        let status = support::websocket_server::receive_request(&mut socket).await;
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
        support::websocket_server::send_result(
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

    let client = Client::builder(url)
        .auth(Auth::none())
        .connect()
        .await
        .unwrap();
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
    let (url, listener) = support::websocket_server::bind().await;
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
