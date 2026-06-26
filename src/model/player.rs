use std::fmt;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;
use uuid::Uuid;

/// A local validation failure while building an MCSMP model.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum ModelError {
    /// A player was created without either a UUID or a non-blank name.
    #[error("a player reference requires an id, a non-blank name, or both")]
    MissingPlayerIdentity,
    /// A text property that must carry content was blank.
    #[error("{field} must not be blank")]
    BlankField {
        /// The protocol field whose value was blank.
        field: &'static str,
    },
    /// An operator permission level was outside the protocol's valid range.
    #[error("operator permission level must be between 0 and 4, got {value}")]
    InvalidOperatorPermissionLevel {
        /// The rejected permission level.
        value: i32,
    },
    /// An IP-ban request omitted both a direct IP address and player selector.
    #[error("an incoming IP ban requires an ip address, a player selector, or both")]
    MissingIpBanTarget,
}

/// A player selector accepted by MCSMP requests and returned by server
/// responses.
///
/// A selector always contains a UUID, a non-blank name, or both. The server
/// resolves selectors according to its own player data; the client does not
/// perform online-mode or name-format validation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PlayerRef {
    id: Option<Uuid>,
    name: Option<String>,
}

impl PlayerRef {
    /// Creates a selector from optional UUID and name components.
    ///
    /// The call fails when both components are absent, or when the supplied
    /// name is blank.
    pub fn new(id: Option<Uuid>, name: Option<String>) -> Result<Self, ModelError> {
        if let Some(name) = name.as_deref() {
            ensure_not_blank("name", name)?;
        }
        if id.is_none() && name.is_none() {
            return Err(ModelError::MissingPlayerIdentity);
        }
        Ok(Self { id, name })
    }

    /// Creates a selector that identifies a player by UUID.
    pub const fn by_id(id: Uuid) -> Self {
        Self {
            id: Some(id),
            name: None,
        }
    }

    /// Creates a selector that identifies a player by name.
    pub fn by_name(name: impl Into<String>) -> Result<Self, ModelError> {
        Self::new(None, Some(name.into()))
    }

    /// Creates a selector containing both UUID and name.
    pub fn both(id: Uuid, name: impl Into<String>) -> Result<Self, ModelError> {
        Self::new(Some(id), Some(name.into()))
    }

    /// Returns the player's UUID when this selector contains one.
    pub const fn id(&self) -> Option<Uuid> {
        self.id
    }

    /// Returns the player's name when this selector contains one.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns whether this selector contains a UUID.
    pub const fn has_id(&self) -> bool {
        self.id.is_some()
    }

    /// Returns whether this selector contains a player name.
    pub const fn has_name(&self) -> bool {
        self.name.is_some()
    }

    /// Returns a display-friendly player identifier, preferring the name.
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
