use mcsmp_rs::{GameRuleKind, GameRuleValue, TypedGameRule, UntypedGameRule};
use serde_json::{json, Value};

#[test]
fn decodes_current_and_legacy_gamerule_scalars() {
    let rules: Vec<TypedGameRule> =
        serde_json::from_str(include_str!("fixtures/gamerules.json")).expect("fixture must decode");

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
