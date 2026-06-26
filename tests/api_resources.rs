use std::net::IpAddr;

use futures_util::{SinkExt, StreamExt};
use mcsmp_rs::{
    Auth, Client, IncomingIpBan, IpBan, KickPlayer, Message, Operator, PlayerRef, SystemMessage,
    UserBan,
};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;
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
            let WsMessage::Text(text) = frame else {
                continue;
            };
            let request: Value = serde_json::from_str(text.as_ref()).unwrap();
            let method = request["method"].as_str().unwrap();
            let result = result_for(method);
            socket
                .send(WsMessage::Text(
                    json!({"jsonrpc": "2.0", "id": request["id"], "result": result})
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

fn result_for(method: &str) -> Value {
    match method {
        "minecraft:allowlist"
        | "minecraft:allowlist/set"
        | "minecraft:allowlist/add"
        | "minecraft:allowlist/remove"
        | "minecraft:allowlist/clear" => json!({"allowlist": []}),
        "minecraft:bans"
        | "minecraft:bans/set"
        | "minecraft:bans/add"
        | "minecraft:bans/remove"
        | "minecraft:bans/clear" => json!({"banlist": []}),
        "minecraft:ip_bans"
        | "minecraft:ip_bans/set"
        | "minecraft:ip_bans/add"
        | "minecraft:ip_bans/remove"
        | "minecraft:ip_bans/clear" => json!({"banlist": []}),
        "minecraft:operators"
        | "minecraft:operators/set"
        | "minecraft:operators/add"
        | "minecraft:operators/remove"
        | "minecraft:operators/clear" => json!({"operators": []}),
        "minecraft:players" => json!({"players": []}),
        "minecraft:players/kick" => json!({"kicked": []}),
        "minecraft:server/status" => json!({
            "status": {"started": true, "players": [], "version": {"name": "26.2", "protocol": 1}}
        }),
        "minecraft:server/save" => json!({"saving": true}),
        "minecraft:server/stop" => json!({"stopping": true}),
        "minecraft:server/system_message" => json!({"sent": true}),
        unexpected => panic!("unexpected MCSMP method: {unexpected}"),
    }
}

#[tokio::test]
async fn typed_resource_api_maps_official_endpoints_and_payloads() {
    const REQUEST_COUNT: usize = 26;
    let (url, server) = start_mock_server(REQUEST_COUNT).await;
    let client = Client::builder(url)
        .auth(Auth::none())
        .connect()
        .await
        .unwrap();

    let alex = PlayerRef::by_name("Alex").unwrap();
    let ip: IpAddr = "203.0.113.8".parse().unwrap();
    let user_ban = UserBan::with_reason(alex.clone(), "Abuse").unwrap();
    let ip_ban = IpBan::with_reason(ip, "Abuse").unwrap();
    let incoming_ip_ban = IncomingIpBan::ip(ip);
    let operator = Operator::with_options(alex.clone(), 4, true).unwrap();

    client.allowlist().list().await.unwrap();
    client.allowlist().set([alex.clone()]).await.unwrap();
    client.allowlist().add([alex.clone()]).await.unwrap();
    client.allowlist().remove([alex.clone()]).await.unwrap();
    client.allowlist().clear().await.unwrap();

    client.bans().list().await.unwrap();
    client.bans().set([user_ban.clone()]).await.unwrap();
    client.bans().add([user_ban]).await.unwrap();
    client.bans().remove([alex.clone()]).await.unwrap();
    client.bans().clear().await.unwrap();

    client.ip_bans().list().await.unwrap();
    client.ip_bans().set([ip_ban]).await.unwrap();
    client.ip_bans().add([incoming_ip_ban]).await.unwrap();
    client.ip_bans().remove([ip]).await.unwrap();
    client.ip_bans().clear().await.unwrap();

    client.operators().list().await.unwrap();
    client.operators().set([operator.clone()]).await.unwrap();
    client.operators().add([operator]).await.unwrap();
    client.operators().remove([alex.clone()]).await.unwrap();
    client.operators().clear().await.unwrap();

    client.players().list().await.unwrap();
    client
        .players()
        .kick([KickPlayer::with_message(
            alex.clone(),
            Message::literal("Bye"),
        )])
        .await
        .unwrap();

    assert!(client.server().status().await.unwrap().started);
    assert!(client.server().save(true).await.unwrap());
    assert!(client.server().stop().await.unwrap());
    assert!(
        client
            .server()
            .system_message(SystemMessage::chat(Message::literal("Hello")))
            .await
            .unwrap()
    );

    let requests = server.await.unwrap();
    assert_eq!(requests.len(), REQUEST_COUNT);
    assert_eq!(
        requests
            .iter()
            .map(|request| request["method"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec![
            "minecraft:allowlist",
            "minecraft:allowlist/set",
            "minecraft:allowlist/add",
            "minecraft:allowlist/remove",
            "minecraft:allowlist/clear",
            "minecraft:bans",
            "minecraft:bans/set",
            "minecraft:bans/add",
            "minecraft:bans/remove",
            "minecraft:bans/clear",
            "minecraft:ip_bans",
            "minecraft:ip_bans/set",
            "minecraft:ip_bans/add",
            "minecraft:ip_bans/remove",
            "minecraft:ip_bans/clear",
            "minecraft:operators",
            "minecraft:operators/set",
            "minecraft:operators/add",
            "minecraft:operators/remove",
            "minecraft:operators/clear",
            "minecraft:players",
            "minecraft:players/kick",
            "minecraft:server/status",
            "minecraft:server/save",
            "minecraft:server/stop",
            "minecraft:server/system_message",
        ]
    );
    assert_eq!(requests[2]["params"], json!({"add": [{"name": "Alex"}]}));
    assert_eq!(requests[7]["params"]["add"][0]["reason"], "Abuse");
    assert_eq!(
        requests[12]["params"],
        json!({"add": [{"ip": "203.0.113.8"}]})
    );
    assert_eq!(requests[13]["params"], json!({"ip": ["203.0.113.8"]}));
    assert_eq!(requests[17]["params"]["add"][0]["permissionLevel"], 4);
    assert_eq!(
        requests[21]["params"]["kick"][0]["message"],
        json!({"literal": "Bye"})
    );
    assert_eq!(requests[23]["params"], json!({"flush": true}));
    assert_eq!(
        requests[25]["params"],
        json!({"message": {"message": {"literal": "Hello"}, "overlay": false}})
    );

    client.shutdown().await.unwrap();
}
