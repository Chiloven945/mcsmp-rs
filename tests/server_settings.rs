use futures_util::{SinkExt, StreamExt};
use mcsmp_rs::{Auth, Client, Difficulty, GameMode};
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
async fn server_settings_api_maps_every_getter_and_setter() {
    const REQUESTS: usize = 40;
    let (url, server) = start_mock_server(REQUESTS).await;
    let client = Client::builder(url)
        .auth(Auth::none())
        .connect()
        .await
        .unwrap();
    let settings = client.server_settings();

    assert!(settings.autosave().await.unwrap());
    assert!(!settings.set_autosave(false).await.unwrap());
    assert_eq!(settings.difficulty().await.unwrap(), Difficulty::Hard);
    assert_eq!(
        settings.set_difficulty(Difficulty::Easy).await.unwrap(),
        Difficulty::Easy
    );
    assert!(settings.enforce_allowlist().await.unwrap());
    assert!(!settings.set_enforce_allowlist(false).await.unwrap());
    assert!(settings.use_allowlist().await.unwrap());
    assert!(!settings.set_use_allowlist(false).await.unwrap());
    assert_eq!(settings.max_players().await.unwrap(), 20);
    assert_eq!(settings.set_max_players(24).await.unwrap(), 24);
    assert_eq!(settings.pause_when_empty_seconds().await.unwrap(), 60);
    assert_eq!(settings.set_pause_when_empty_seconds(30).await.unwrap(), 30);
    assert_eq!(settings.player_idle_timeout().await.unwrap(), 120);
    assert_eq!(settings.set_player_idle_timeout(90).await.unwrap(), 90);
    assert!(!settings.allow_flight().await.unwrap());
    assert!(settings.set_allow_flight(true).await.unwrap());
    assert_eq!(settings.motd().await.unwrap(), "Welcome");
    assert_eq!(settings.set_motd("New MOTD").await.unwrap(), "New MOTD");
    assert_eq!(settings.spawn_protection_radius().await.unwrap(), 16);
    assert_eq!(settings.set_spawn_protection_radius(12).await.unwrap(), 12);
    assert!(!settings.force_game_mode().await.unwrap());
    assert!(settings.set_force_game_mode(true).await.unwrap());
    assert_eq!(settings.game_mode().await.unwrap(), GameMode::Survival);
    assert_eq!(
        settings.set_game_mode(GameMode::Creative).await.unwrap(),
        GameMode::Creative
    );
    assert_eq!(settings.view_distance().await.unwrap(), 10);
    assert_eq!(settings.set_view_distance(12).await.unwrap(), 12);
    assert_eq!(settings.simulation_distance().await.unwrap(), 8);
    assert_eq!(settings.set_simulation_distance(10).await.unwrap(), 10);
    assert!(settings.accept_transfers().await.unwrap());
    assert!(!settings.set_accept_transfers(false).await.unwrap());
    assert_eq!(settings.status_heartbeat_interval().await.unwrap(), 5);
    assert_eq!(
        settings.set_status_heartbeat_interval(10).await.unwrap(),
        10
    );
    assert_eq!(settings.operator_user_permission_level().await.unwrap(), 2);
    assert_eq!(
        settings
            .set_operator_user_permission_level(3)
            .await
            .unwrap(),
        3
    );
    assert!(!settings.hide_online_players().await.unwrap());
    assert!(settings.set_hide_online_players(true).await.unwrap());
    assert!(settings.status_replies().await.unwrap());
    assert!(!settings.set_status_replies(false).await.unwrap());
    assert_eq!(settings.entity_broadcast_range().await.unwrap(), 100);
    assert_eq!(settings.set_entity_broadcast_range(75).await.unwrap(), 75);

    let requests = server.await.unwrap();
    assert_eq!(requests.len(), REQUESTS);
    assert_eq!(requests[1]["params"], json!({"enable": false}));
    assert_eq!(requests[3]["params"], json!({"difficulty": "easy"}));
    assert_eq!(requests[35]["params"], json!({"hide": true}));
    assert_eq!(requests[39]["params"], json!({"percentage_points": 75}));

    client.shutdown().await.unwrap();
}
