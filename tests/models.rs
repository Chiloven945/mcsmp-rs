use std::net::IpAddr;

use mcsmp_rs::{
    IncomingIpBan, IpBan, Message, Operator, PlayerRef, ServerState, SystemMessage, UserBan,
};
use serde_json::{json, Value};
use uuid::Uuid;

#[test]
fn deserializes_server_status_fixture() {
    let state: ServerState = serde_json::from_str(include_str!("fixtures/model/server_state.json"))
        .expect("fixture must decode");

    assert!(state.started);
    assert_eq!(state.online_player_count(), 1);
    assert_eq!(state.players[0].name(), Some("Alex"));
    assert_eq!(state.version.name, "26.2");
}

#[test]
fn ban_fixture_uses_protocol_field_names() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/model/bans.json"))
        .expect("fixture must be valid JSON");
    let user: UserBan = serde_json::from_value(fixture["user"].clone()).expect("user ban");
    let ip: IpBan = serde_json::from_value(fixture["ip"].clone()).expect("ip ban");
    let operator: Operator = serde_json::from_value(fixture["operator"].clone()).expect("operator");

    assert_eq!(user.player.name(), Some("Alex"));
    assert_eq!(user.reason.as_deref(), Some("Repeated griefing"));
    assert_eq!(ip.ip, "203.0.113.8".parse::<IpAddr>().unwrap());
    assert_eq!(operator.permission_level, Some(4));
    assert_eq!(operator.bypasses_player_limit, Some(true));

    assert_eq!(
        serde_json::to_value(&operator).unwrap(),
        fixture["operator"].clone()
    );
}

#[test]
fn translatable_message_precedes_literal_fallback() {
    let message: Message = serde_json::from_str(include_str!("fixtures/model/messages.json"))
        .expect("fixture must decode");

    assert_eq!(message.translation_key(), Some("chat.type.announcement"));
    assert_eq!(message.literal_text(), None);
    assert_eq!(
        serde_json::to_value(&message).unwrap(),
        json!({
            "translatable": "chat.type.announcement",
            "translatableParams": ["Server", "Restarting"]
        })
    );
}

#[test]
fn constructors_emit_mcsmp_wire_shapes() {
    let id = Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").unwrap();
    let player = PlayerRef::both(id, "Alex").unwrap();
    let ban = IncomingIpBan::ip("203.0.113.8".parse().unwrap())
        .with_reason("Abuse")
        .unwrap();
    let message = SystemMessage::action_bar(Message::literal("Maintenance")).to([player.clone()]);

    assert_eq!(
        serde_json::to_value(player).unwrap(),
        json!({"id": "123e4567-e89b-12d3-a456-426614174000", "name": "Alex"})
    );
    assert_eq!(
        serde_json::to_value(ban).unwrap(),
        json!({"ip": "203.0.113.8", "reason": "Abuse"})
    );
    assert_eq!(
        serde_json::to_value(message).unwrap(),
        json!({
            "message": {"literal": "Maintenance"},
            "overlay": true,
            "receivingPlayers": [{"id": "123e4567-e89b-12d3-a456-426614174000", "name": "Alex"}]
        })
    );
}

#[test]
fn rejects_invalid_required_selectors() {
    assert!(PlayerRef::new(None, None).is_err());
    assert!(PlayerRef::by_name("  ").is_err());
    assert!(IncomingIpBan::new(None, None).is_err());
    assert!(serde_json::from_value::<PlayerRef>(json!({})).is_err());
    assert!(serde_json::from_value::<IncomingIpBan>(json!({})).is_err());
}
