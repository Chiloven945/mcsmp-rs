use std::fmt;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;
use uuid::Uuid;

/// Local validation failure while constructing an MCSMP request or model.
///
/// These errors occur before a request is sent. The enum is `non_exhaustive`;
/// downstream pattern matches should include a fallback arm.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum ModelError {
    /// A player selector omitted both UUID and non-blank name.
    #[error("a player reference requires an id, a non-blank name, or both")]
    MissingPlayerIdentity,
    /// A required text property was empty or whitespace-only.
    #[error("{field} must not be blank")]
    BlankField {
        /// Protocol field name whose supplied text was blank.
        field: &'static str,
    },
    /// An operator permission level was outside Minecraft's supported `0..=4` range.
    #[error("operator permission level must be between 0 and 4, got {value}")]
    InvalidOperatorPermissionLevel {
        /// Permission level rejected by the local model validator.
        value: i32,
    },
    /// An IP-ban input omitted both a direct address and a player selector.
    #[error("an incoming IP ban requires an ip address, a player selector, or both")]
    MissingIpBanTarget,
    /// A native gamerule scalar did not match the server-declared gamerule kind.
    ///
    /// Legacy string values are intentionally allowed because older MCSMP
    /// variants could not express a native type.
    #[error("gamerule value type mismatch: expected {expected}, got {actual}")]
    GameRuleTypeMismatch {
        /// Scalar kind required by the gamerule declaration.
        expected: &'static str,
        /// Scalar kind actually supplied by the gamerule value.
        actual: &'static str,
    },
}

/// UUID and/or name selector for a Minecraft player.
///
/// MCSMP accepts player references in many management operations. A selector
/// must contain at least one identity component: a UUID, a non-blank player
/// name, or both. When both are present, Minecraft resolves the selector using
/// its own player data; this crate deliberately does not perform online-mode,
/// account-existence, or username-format validation.
///
/// Serialization omits absent fields, producing `{ "id": ... }`,
/// `{ "name": ... }`, or both. Use [`Self::by_id`] where a stable identity is
/// available and [`Self::by_name`] for human-entered administrative commands.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PlayerRef {
    id: Option<Uuid>,
    name: Option<String>,
}

impl PlayerRef {
    /// Creates a selector from optional UUID and name components.
    ///
    /// Returns [`ModelError::MissingPlayerIdentity`] when both inputs are
    /// `None`, and [`ModelError::BlankField`] when a provided name is empty or
    /// whitespace-only. The name is otherwise preserved exactly as supplied.
    pub fn new(id: Option<Uuid>, name: Option<String>) -> Result<Self, ModelError> {
        if let Some(name) = name.as_deref() {
            ensure_not_blank("name", name)?;
        }
        if id.is_none() && name.is_none() {
            return Err(ModelError::MissingPlayerIdentity);
        }
        Ok(Self { id, name })
    }

    /// Creates a selector that identifies a player exclusively by UUID.
    ///
    /// UUID selectors are stable across username changes and are generally
    /// preferred when an application has previously resolved the account.
    pub const fn by_id(id: Uuid) -> Self {
        Self {
            id: Some(id),
            name: None,
        }
    }

    /// Creates a selector that identifies a player by non-blank name.
    ///
    /// Names are resolved by Minecraft at request time. This method returns
    /// [`ModelError::BlankField`] for empty or whitespace-only input.
    pub fn by_name(name: impl Into<String>) -> Result<Self, ModelError> {
        Self::new(None, Some(name.into()))
    }

    /// Creates a selector containing both UUID and non-blank name.
    ///
    /// Supplying both values can improve diagnostics while retaining stable
    /// UUID identity. The server remains the authority for resolving any
    /// disagreement between stored account data and the provided name.
    pub fn both(id: Uuid, name: impl Into<String>) -> Result<Self, ModelError> {
        Self::new(Some(id), Some(name.into()))
    }

    /// Returns this selector's UUID, when one was provided or returned.
    pub const fn id(&self) -> Option<Uuid> {
        self.id
    }

    /// Returns this selector's player name, when one was provided or returned.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns `true` when this selector contains a UUID component.
    pub const fn has_id(&self) -> bool {
        self.id.is_some()
    }

    /// Returns `true` when this selector contains a player-name component.
    pub const fn has_name(&self) -> bool {
        self.name.is_some()
    }

    /// Returns a human-friendly identifier, preferring name over UUID.
    ///
    /// The result is suitable for logs and UI labels. It is not guaranteed to
    /// be unique when a selector only carries a name; use [`Self::id`] when
    /// stable identity is required.
    pub fn display_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| {
            self.id
                .expect("validated PlayerRef always has an id or name")
                .to_string()
        })
    }
}

impl Serialize for PlayerRef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Wire<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            id: Option<Uuid>,
            #[serde(skip_serializing_if = "Option::is_none")]
            name: Option<&'a str>,
        }

        Wire {
            id: self.id,
            name: self.name.as_deref(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PlayerRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            id: Option<Uuid>,
            name: Option<String>,
        }

        let wire = Wire::deserialize(deserializer)?;
        Self::new(wire.id, wire.name).map_err(D::Error::custom)
    }
}

impl fmt::Display for PlayerRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.display_name())
    }
}

pub(crate) fn ensure_not_blank(field: &'static str, value: &str) -> Result<(), ModelError> {
    if value.trim().is_empty() {
        Err(ModelError::BlankField { field })
    } else {
        Ok(())
    }
}
