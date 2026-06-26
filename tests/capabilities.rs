use futures_util::{SinkExt, StreamExt};
use mcsmp_rs::{Auth, Client, CompatibilityMode, Error, Feature, ProtocolVersion};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

async fn start_mock_server(expected_requests: usize) -> (Url, JoinHandle<Vec<Value>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        let mut requests = Vec::with_capacity(expected_requests);

        while requests.len() < expected_requests {
            let frame = socket.next().await.unwrap().unwrap();
            let Message::Text(text) = frame else {
                continue;
            };
            let request: Value = serde_json::from_str(text.as_ref()).unwrap();
            let result = result_for(&request);
            socket
                .send(Message::Text(
                    json!({"jsonrpc":"2.0", "id":request["id"], "result":result})
                        .to_string()
                        .into(),
                ))
                .await
                .unwrap();
            requests.push(request);
        }
        requests
    });

    (Url::parse(&format!("ws://{address}")).unwrap(), task)
}

fn result_for(request: &Value) -> Value {
    let method = request["method"].as_str().unwrap();
    let params = &request["params"];
    match method {
        "minecraft:serversettings/autosave" => json!({"enabled": true}),
        "minecraft:serversettings/autosave/set" => json!({"enabled": params["enable"]}),
        "minecraft:serversettings/difficulty" => json!({"difficulty": "hard"}),
        "minecraft:serversettings/difficulty/set" => json!({"difficulty": params["difficulty"]}),
        "minecraft:serversettings/enforce_allowlist" => json!({"enforced": true}),
        "minecraft:serversettings/enforce_allowlist/set" => json!({"enforced": params["enforce"]}),
        "minecraft:serversettings/use_allowlist" => json!({"used": true}),
        "minecraft:serversettings/use_allowlist/set" => json!({"used": params["use"]}),
        "minecraft:serversettings/max_players" => json!({"max": 20}),
        "minecraft:serversettings/max_players/set" => json!({"max": params["max"]}),
        "minecraft:serversettings/pause_when_empty_seconds" => json!({"seconds": 60}),
        "minecraft:serversettings/pause_when_empty_seconds/set" => {
            json!({"seconds": params["seconds"]})
        }
        "minecraft:serversettings/player_idle_timeout" => json!({"seconds": 120}),
        "minecraft:serversettings/player_idle_timeout/set" => json!({"seconds": params["seconds"]}),
        "minecraft:serversettings/allow_flight" => json!({"allowed": false}),
        "minecraft:serversettings/allow_flight/set" => json!({"allowed": params["allowed"]}),
        "minecraft:serversettings/motd" => json!({"message": "Welcome"}),
        "minecraft:serversettings/motd/set" => json!({"message": params["message"]}),
        "minecraft:serversettings/spawn_protection_radius" => json!({"radius": 16}),
        "minecraft:serversettings/spawn_protection_radius/set" => {
            json!({"radius": params["radius"]})
        }
        "minecraft:serversettings/force_game_mode" => json!({"forced": false}),
        "minecraft:serversettings/force_game_mode/set" => json!({"forced": params["force"]}),
        "minecraft:serversettings/game_mode" => json!({"mode": "survival"}),
        "minecraft:serversettings/game_mode/set" => json!({"mode": params["mode"]}),
        "minecraft:serversettings/view_distance" => json!({"distance": 10}),
        "minecraft:serversettings/view_distance/set" => json!({"distance": params["distance"]}),
        "minecraft:serversettings/simulation_distance" => json!({"distance": 8}),
        "minecraft:serversettings/simulation_distance/set" => {
            json!({"distance": params["distance"]})
        }
        "minecraft:serversettings/accept_transfers" => json!({"accepted": true}),
        "minecraft:serversettings/accept_transfers/set" => json!({"accepted": params["accept"]}),
        "minecraft:serversettings/status_heartbeat_interval" => json!({"seconds": 5}),
        "minecraft:serversettings/status_heartbeat_interval/set" => {
            json!({"seconds": params["seconds"]})
        }
        "minecraft:serversettings/operator_user_permission_level" => json!({"level": 2}),
        "minecraft:serversettings/operator_user_permission_level/set" => {
            json!({"level": params["level"]})
        }
        "minecraft:serversettings/hide_online_players" => json!({"hidden": false}),
        "minecraft:serversettings/hide_online_players/set" => json!({"hidden": params["hide"]}),
        "minecraft:serversettings/status_replies" => json!({"enabled": true}),
        "minecraft:serversettings/status_replies/set" => json!({"enabled": params["enable"]}),
        "minecraft:serversettings/entity_broadcast_range" => json!({"percentage_points": 100}),
        "minecraft:serversettings/entity_broadcast_range/set" => {
            json!({"percentage_points": params["percentage_points"]})
        }
        "minecraft:gamerules" => json!({
            "gamerules": [
                {"key":"doDaylightCycle","type":"boolean","value":true},
                {"key":"randomTickSpeed","type":"integer","value":3}
            ]
        }),
        "minecraft:gamerules/update" => {
            let gamerule = &params["gamerule"];
            let kind = if gamerule["value"].is_boolean() {
                "boolean"
            } else {
                "integer"
            };
            json!({"gamerule":{"key":gamerule["key"],"type":kind,"value":gamerule["value"]}})
        }
        "rpc.discover" => json!({
            "protocolVersion": "3.1.0",
            "methods": [
                {"name":"rpc.discover"},
                {"name":"minecraft:server/status"}
            ],
            "notifications": {
                "minecraft:notification/server/activity": {},
                "minecraft:notification/world/upgrade_started": {}
            }
        }),
        "minecraft:server/status" => json!({
            "status": {"started": false, "players": [], "version": {"name":"26.2", "protocol":1}}
        }),
        unexpected => panic!("unexpected MCSMP method: {unexpected}"),
    }
}

#[tokio::test]
async fn strict_invocation_uses_discovered_capabilities() {
    let (url, server) = start_mock_server(2).await;
    let client = Client::builder(url)
        .auth(Auth::none())
        .compatibility_mode(CompatibilityMode::Strict)
        .connect()
        .await
        .unwrap();

    assert!(matches!(
        client.server().status().await,
        Err(Error::DiscoveryRequired)
    ));

    let capabilities = client.discover().await.unwrap();
    assert_eq!(capabilities.protocol_version, Some(ProtocolVersion::V3_1_0));
    assert!(capabilities.supports_feature(Feature::WorldUpgradeNotifications));
    assert_eq!(client.capabilities(), Some(capabilities.clone()));

    let status = client.server().status().await.unwrap();
    assert!(!status.started);
    assert!(matches!(
        client.players().list().await,
        Err(Error::UnsupportedMethod { .. })
    ));

    let requests = server.await.unwrap();
    assert_eq!(
        requests
            .iter()
            .map(|request| request["method"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["rpc.discover", "minecraft:server/status"]
    );
    client.shutdown().await.unwrap();
}
