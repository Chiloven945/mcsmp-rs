use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use super::ModelError;
use super::player::ensure_not_blank;

/// Declared native scalar kind of a Minecraft gamerule.
///
/// The server includes this value in [`TypedGameRule`] responses. It is used to
/// validate native boolean and integer values. Older servers can still return
/// string values, represented by [`GameRuleValue::LegacyString`] even when a
/// declared kind is present.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum GameRuleKind {
    /// Gamerule normally holds a signed integer value.
    Integer,
    /// Gamerule normally holds a boolean value.
    Boolean,
}

/// Scalar gamerule value compatible with current and legacy MCSMP servers.
///
/// MCSMP 2.0 uses native JSON booleans and signed integers. Earlier
/// experimental versions and custom servers can still send JSON strings. Those
/// strings are represented losslessly as [`Self::LegacyString`] rather than
/// being guessed as booleans or integers.
///
/// This distinction matters for correctness: `"true"` remains a string, while
/// `"12"` can be explicitly converted with [`Self::parse_integer`] when an
/// application chooses to support that legacy convention.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GameRuleValue {
    /// Native JSON boolean gamerule value.
    Boolean(bool),
    /// Native JSON signed 32-bit integer gamerule value.
    Integer(i32),
    /// Legacy JSON string gamerule value preserved without implicit coercion.
    LegacyString(String),
}

impl GameRuleValue {
    /// Creates a native boolean gamerule value.
    pub const fn boolean(value: bool) -> Self {
        Self::Boolean(value)
    }

    /// Creates a native signed 32-bit integer gamerule value.
    pub const fn integer(value: i32) -> Self {
        Self::Integer(value)
    }

    /// Creates a legacy string gamerule value without validation or coercion.
    pub fn legacy_string(value: impl Into<String>) -> Self {
        Self::LegacyString(value.into())
    }

    /// Returns the native boolean when this value is [`Self::Boolean`].
    ///
    /// String values, including `"true"` and `"false"`, return `None`.
    pub const fn as_boolean(&self) -> Option<bool> {
        match self {
            Self::Boolean(value) => Some(*value),
            Self::Integer(_) | Self::LegacyString(_) => None,
        }
    }

    /// Returns the native integer when this value is [`Self::Integer`].
    ///
    /// Legacy numeric strings return `None`; use [`Self::parse_integer`] when
    /// an explicit fallback conversion is desired.
    pub const fn as_integer(&self) -> Option<i32> {
        match self {
            Self::Integer(value) => Some(*value),
            Self::Boolean(_) | Self::LegacyString(_) => None,
        }
    }

    /// Returns the stored string when this value is [`Self::LegacyString`].
    pub fn as_legacy_string(&self) -> Option<&str> {
        match self {
            Self::LegacyString(value) => Some(value),
            Self::Boolean(_) | Self::Integer(_) => None,
        }
    }

    /// Returns a signed 32-bit integer from a native or legacy numeric value.
    ///
    /// Native integers are returned directly. Legacy strings are parsed with
    /// Rust's `i32` parser. This intentionally does not parse `"true"` or
    /// `"false"`: legacy boolean-looking strings are not equivalent to native
    /// boolean protocol values.
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

/// Gamerule returned by the server together with its declared native kind.
///
/// The wire representation contains `key`, `type`, and `value`. Native
/// booleans and integers must agree with `kind`; otherwise deserialization or
/// construction returns [`ModelError::GameRuleTypeMismatch`]. Legacy string
/// values are accepted to preserve compatibility with older server wire forms.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TypedGameRule {
    /// Non-blank gamerule resource key, such as `doDaylightCycle`.
    pub key: String,
    /// Native scalar kind supplied in the wire field named `type`.
    #[serde(rename = "type")]
    pub kind: GameRuleKind,
    /// Current native or legacy scalar value supplied by the server.
    pub value: GameRuleValue,
}

impl TypedGameRule {
    /// Creates and validates a typed gamerule.
    ///
    /// `key` must be non-blank. A native boolean must use
    /// [`GameRuleKind::Boolean`] and a native integer must use
    /// [`GameRuleKind::Integer`]. Legacy string values are accepted with either
    /// kind because historical MCSMP payloads cannot express the native type.
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

/// Gamerule update request without a server-declared kind.
///
/// MCSMP update requests intentionally contain only `key` and `value`; the
/// server owns the authoritative kind. Use the convenience constructors when
/// the desired scalar type is known.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UntypedGameRule {
    /// Non-blank gamerule key to update.
    pub key: String,
    /// New native or legacy scalar value to send.
    pub value: GameRuleValue,
}

impl UntypedGameRule {
    /// Creates a gamerule update request from a non-blank key and scalar value.
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

    /// Creates an update request with a native boolean scalar.
    pub fn boolean(key: impl Into<String>, value: bool) -> Result<Self, ModelError> {
        Self::new(key, GameRuleValue::Boolean(value))
    }

    /// Creates an update request with a native signed integer scalar.
    pub fn integer(key: impl Into<String>, value: i32) -> Result<Self, ModelError> {
        Self::new(key, GameRuleValue::Integer(value))
    }

    /// Creates an update request with a legacy string scalar.
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
