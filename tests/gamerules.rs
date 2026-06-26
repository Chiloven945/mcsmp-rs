use mcsmp_rs::{GameRuleKind, GameRuleValue, TypedGameRule, UntypedGameRule};
use serde_json::{json, Value};

#[test]
fn decodes_current_and_legacy_gamerule_scalars() {
    let rules: Vec<TypedGameRule> =
        serde_json::from_str(include_str!("fixtures/model/gamerules.json"))
            .expect("fixture must decode");

    assert_eq!(rules[0].value.as_boolean(), Some(true));
    assert_eq!(rules[1].value.as_integer(), Some(3));
    assert_eq!(rules[2].value.as_legacy_string(), Some("12"));
    assert_eq!(rules[2].value.parse_integer(), Some(12));
    assert_eq!(
        GameRuleValue::legacy_string("true").parse_integer(),
        None,
        "legacy boolean-looking strings must not become booleans"
    );
}

#[test]
fn serializes_update_with_scalar_not_enum_wrapper() {
    let boolean = UntypedGameRule::boolean("doDaylightCycle", false).unwrap();
    let integer = UntypedGameRule::integer("randomTickSpeed", 8).unwrap();
    let legacy = UntypedGameRule::legacy_string("legacyCounter", "12").unwrap();

    assert_eq!(
        serde_json::to_value(boolean).unwrap(),
        json!({"key":"doDaylightCycle","value":false})
    );
    assert_eq!(
        serde_json::to_value(integer).unwrap(),
        json!({"key":"randomTickSpeed","value":8})
    );
    assert_eq!(
        serde_json::to_value(legacy).unwrap(),
        json!({"key":"legacyCounter","value":"12"})
    );
}

#[test]
fn rejects_mismatched_native_typed_value() {
    let malformed: Value = json!({"key":"randomTickSpeed","type":"integer","value":true});
    assert!(serde_json::from_value::<TypedGameRule>(malformed).is_err());

    assert!(
        TypedGameRule::new(
            "doDaylightCycle",
            GameRuleKind::Boolean,
            GameRuleValue::Boolean(true),
        )
        .is_ok()
    );
}

use futures_util::{SinkExt, StreamExt};
use mcsmp_rs::{Auth, Client};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

#[tokio::test]
async fn gamerule_api_uses_list_and_update_wire_contracts() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = Url::parse(&format!("ws://{}", listener.local_addr().unwrap())).unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        let mut requests = Vec::new();
        while requests.len() < 3 {
            let Message::Text(text) = socket.next().await.unwrap().unwrap() else {
                continue;
            };
            let request: Value = serde_json::from_str(text.as_ref()).unwrap();
            let result = match request["method"].as_str().unwrap() {
                "minecraft:gamerules" => {
                    json!({"gamerules":[{"key":"doDaylightCycle","type":"boolean","value":true}]})
                }
                "minecraft:gamerules/update" => {
                    json!({"gamerule":{"key":request["params"]["gamerule"]["key"],"type":"integer","value":request["params"]["gamerule"]["value"]}})
                }
                method => panic!("unexpected method: {method}"),
            };
            socket
                .send(Message::Text(
                    json!({"jsonrpc":"2.0","id":request["id"],"result":result})
                        .to_string()
                        .into(),
                ))
                .await
                .unwrap();
            requests.push(request);
        }
        requests
    });
    let client = Client::builder(url)
        .auth(Auth::none())
        .connect()
        .await
        .unwrap();
    assert_eq!(
        client.gamerules().list().await.unwrap()[0].value,
        GameRuleValue::Boolean(true)
    );
    let updated = client
        .gamerules()
        .update(UntypedGameRule::boolean("doDaylightCycle", false).unwrap())
        .await
        .unwrap();
    assert_eq!(updated.value, GameRuleValue::Boolean(false));
    let legacy = client
        .gamerules()
        .update(UntypedGameRule::legacy_string("legacyCounter", "12").unwrap())
        .await
        .unwrap();
    assert_eq!(legacy.value, GameRuleValue::LegacyString("12".into()));
    let requests = server.await.unwrap();
    assert_eq!(
        requests[1]["params"],
        json!({"gamerule":{"key":"doDaylightCycle","value":false}})
    );
    client.shutdown().await.unwrap();
}
