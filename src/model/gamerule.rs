use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use super::player::ensure_not_blank;
use super::ModelError;

/// Declared scalar kind of a Minecraft gamerule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum GameRuleKind {
    /// The gamerule holds an integer value.
    Integer,
    /// The gamerule holds a boolean value.
    Boolean,
}

/// A scalar gamerule value compatible with both current and legacy MCSMP
/// servers.
///
/// MCSMP 2.0 uses JSON booleans and integers. Earlier experimental versions
/// and custom servers may still send string values, which are represented by
/// [`GameRuleValue::LegacyString`] without coercing boolean-looking strings.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GameRuleValue {
    /// A native JSON boolean gamerule value.
    Boolean(bool),
    /// A native JSON signed 32-bit integer gamerule value.
    Integer(i32),
    /// A legacy JSON string gamerule value.
    LegacyString(String),
}

impl GameRuleValue {
    /// Creates a boolean gamerule value.
    pub const fn boolean(value: bool) -> Self {
        Self::Boolean(value)
    }

    /// Creates an integer gamerule value.
    pub const fn integer(value: i32) -> Self {
        Self::Integer(value)
    }

    /// Creates a legacy string gamerule value.
    pub fn legacy_string(value: impl Into<String>) -> Self {
        Self::LegacyString(value.into())
    }

    /// Returns the value as a boolean when it is boolean-typed.
    pub const fn as_boolean(&self) -> Option<bool> {
        match self {
            Self::Boolean(value) => Some(*value),
            Self::Integer(_) | Self::LegacyString(_) => None,
        }
    }

    /// Returns the value as an integer when it is integer-typed.
    pub const fn as_integer(&self) -> Option<i32> {
        match self {
            Self::Integer(value) => Some(*value),
            Self::Boolean(_) | Self::LegacyString(_) => None,
        }
    }

    /// Returns the legacy string when this value was decoded from or created as
    /// a string.
    pub fn as_legacy_string(&self) -> Option<&str> {
        match self {
            Self::LegacyString(value) => Some(value),
            Self::Boolean(_) | Self::Integer(_) => None,
        }
    }

    /// Parses a legacy string as a signed 32-bit integer.
    ///
    /// This intentionally does not parse `"true"` or `"false"`: the protocol
    /// permits integer strings for compatibility, but boolean strings are not
    /// equivalent to native boolean values.
    pub fn parse_integer(&self) -> Option<i32> {
        match self {
            Self::Integer(value) => Some(*value),
            Self::LegacyString(value) => value.parse().ok(),
            Self::Boolean(_) => None,
        }
    }

    fn scalar_name(&self) -> &'static str {
        match self {
            Self::Boolean(_) => "boolean",
            Self::Integer(_) => "integer",
            Self::LegacyString(_) => "legacy string",
        }
    }
}

impl From<bool> for GameRuleValue {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<i32> for GameRuleValue {
    fn from(value: i32) -> Self {
        Self::Integer(value)
    }
}

impl From<String> for GameRuleValue {
    fn from(value: String) -> Self {
        Self::LegacyString(value)
    }
}

impl From<&str> for GameRuleValue {
    fn from(value: &str) -> Self {
        Self::LegacyString(value.to_owned())
    }
}

impl Serialize for GameRuleValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Boolean(value) => serializer.serialize_bool(*value),
            Self::Integer(value) => serializer.serialize_i32(*value),
            Self::LegacyString(value) => serializer.serialize_str(value),
        }
    }
}

impl<'de> Deserialize<'de> for GameRuleValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        match value {
            Value::Bool(value) => Ok(Self::Boolean(value)),
            Value::Number(value) => {
                let value = value
                    .as_i64()
                    .and_then(|value| i32::try_from(value).ok())
                    .ok_or_else(|| D::Error::custom("gamerule integer is outside i32 range"))?;
                Ok(Self::Integer(value))
            }
            Value::String(value) => Ok(Self::LegacyString(value)),
            _ => Err(D::Error::custom(
                "gamerule value must be a boolean, signed integer, or string",
            )),
        }
    }
}

/// A gamerule returned by the server together with its declared type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TypedGameRule {
    /// Resource identifier/key of the gamerule.
    pub key: String,
    /// Declared type supplied by the server.
    #[serde(rename = "type")]
    pub kind: GameRuleKind,
    /// Current scalar value.
    pub value: GameRuleValue,
}

impl TypedGameRule {
    /// Creates a typed gamerule, validating native boolean/integer values
    /// against the declared kind. Legacy strings are accepted for older
    /// servers, whose wire format cannot express a native type.
    pub fn new(
        key: impl Into<String>,
        kind: GameRuleKind,
        value: impl Into<GameRuleValue>,
    ) -> Result<Self, ModelError> {
        let key = key.into();
        let value = value.into();
        ensure_not_blank("key", &key)?;
        validate_kind(kind, &value)?;
        Ok(Self { key, kind, value })
    }
}

impl<'de> Deserialize<'de> for TypedGameRule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            key: String,
            #[serde(rename = "type")]
            kind: GameRuleKind,
            value: GameRuleValue,
        }

        let wire = Wire::deserialize(deserializer)?;
        Self::new(wire.key, wire.kind, wire.value).map_err(D::Error::custom)
    }
}

/// A gamerule update request without a declared type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UntypedGameRule {
    /// Resource identifier/key of the gamerule to update.
    pub key: String,
    /// New scalar value for the gamerule.
    pub value: GameRuleValue,
}

impl UntypedGameRule {
    /// Creates a gamerule update request.
    pub fn new(
        key: impl Into<String>,
        value: impl Into<GameRuleValue>,
    ) -> Result<Self, ModelError> {
        let key = key.into();
        ensure_not_blank("key", &key)?;
        Ok(Self {
            key,
            value: value.into(),
        })
    }

    /// Creates a boolean gamerule update request.
    pub fn boolean(key: impl Into<String>, value: bool) -> Result<Self, ModelError> {
        Self::new(key, GameRuleValue::Boolean(value))
    }

    /// Creates an integer gamerule update request.
    pub fn integer(key: impl Into<String>, value: i32) -> Result<Self, ModelError> {
        Self::new(key, GameRuleValue::Integer(value))
    }

    /// Creates a legacy string gamerule update request.
    pub fn legacy_string(
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self, ModelError> {
        Self::new(key, GameRuleValue::LegacyString(value.into()))
    }
}

fn validate_kind(kind: GameRuleKind, value: &GameRuleValue) -> Result<(), ModelError> {
    let valid = matches!(
        (kind, value),
        (GameRuleKind::Boolean, GameRuleValue::Boolean(_))
            | (GameRuleKind::Integer, GameRuleValue::Integer(_))
            | (_, GameRuleValue::LegacyString(_))
    );
    if valid {
        Ok(())
    } else {
        Err(ModelError::GameRuleTypeMismatch {
            expected: match kind {
                GameRuleKind::Boolean => "boolean",
                GameRuleKind::Integer => "integer",
            },
            actual: value.scalar_name(),
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn preserves_legacy_strings_and_parses_integers_explicitly() {
        let value: GameRuleValue = serde_json::from_value(json!("12")).unwrap();
        assert_eq!(value, GameRuleValue::LegacyString("12".into()));
        assert_eq!(value.parse_integer(), Some(12));
        assert_eq!(
            serde_json::from_value::<GameRuleValue>(json!("true"))
                .unwrap()
                .parse_integer(),
            None
        );
    }

    #[test]
    fn validates_native_value_against_kind() {
        assert!(TypedGameRule::new("doDaylightCycle", GameRuleKind::Boolean, true).is_ok());
        assert!(TypedGameRule::new("randomTickSpeed", GameRuleKind::Integer, true).is_err());
    }
}
