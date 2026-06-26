use serde::de::Error as _;
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::PlayerRef;
use super::player::{ModelError, ensure_not_blank};

/// Minecraft display message represented as literal text or a translation key.
///
/// MCSMP encodes literal messages as `{ "literal": "..." }` and translatable
/// messages as `{ "translatable": "key", "translatableParams": [...] }`.
/// Translation-key resolution occurs on the receiving Minecraft client, which
/// can render the same key in each player's language.
///
/// If a payload contains both forms, MCSMP gives the translation form
/// precedence. Deserializing such a payload therefore yields
/// [`Self::Translatable`] and ignores the literal fallback.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Message {
    /// Literal text rendered without translation-key lookup.
    ///
    /// The text is serialized in the protocol's `literal` field.
    Literal(String),
    /// Translation key and string parameters substituted by Minecraft clients.
    Translatable {
        /// Non-blank Minecraft translation key serialized as `translatable`.
        key: String,
        /// Values serialized as `translatableParams` for the translation key.
        params: Vec<String>,
    },
}

impl Message {
    /// Creates a literal message.
    ///
    /// Literal text may be empty because the protocol can intentionally send an
    /// empty display payload. No translation lookup occurs for this variant.
    pub fn literal(text: impl Into<String>) -> Self {
        Self::Literal(text.into())
    }

    /// Creates a translation-key message with string parameters.
    ///
    /// Returns [`ModelError::BlankField`] when `key` is empty or
    /// whitespace-only. Parameters are converted to owned strings in iterator
    /// order and are omitted from the wire payload when the list is empty.
    pub fn translatable<I, P>(key: impl Into<String>, params: I) -> Result<Self, ModelError>
    where
        I: IntoIterator<Item = P>,
        P: Into<String>,
    {
        let key = key.into();
        ensure_not_blank("translatable", &key)?;
        Ok(Self::Translatable {
            key,
            params: params.into_iter().map(Into::into).collect(),
        })
    }

    /// Returns literal text when this is a [`Self::Literal`] message.
    ///
    /// Returns `None` for a translatable message because its final text depends
    /// on the receiving client's language resources.
    pub fn literal_text(&self) -> Option<&str> {
        match self {
            Self::Literal(text) => Some(text),
            Self::Translatable { .. } => None,
        }
    }

    /// Returns the translation key when this is a [`Self::Translatable`] message.
    pub fn translation_key(&self) -> Option<&str> {
        match self {
            Self::Literal(_) => None,
            Self::Translatable { key, .. } => Some(key),
        }
    }
}

impl Serialize for Message {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Literal(text) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("literal", text)?;
                map.end()
            }
            Self::Translatable { key, params } => {
                let entries = if params.is_empty() { 1 } else { 2 };
                let mut map = serializer.serialize_map(Some(entries))?;
                map.serialize_entry("translatable", key)?;
                if !params.is_empty() {
                    map.serialize_entry("translatableParams", params)?;
                }
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for Message {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Wire {
            translatable: Option<String>,
            #[serde(default)]
            translatable_params: Vec<String>,
            literal: Option<String>,
        }

        let wire = Wire::deserialize(deserializer)?;
        match wire.translatable {
            Some(key) => {
                Message::translatable(key, wire.translatable_params).map_err(D::Error::custom)
            }
            None => wire
                .literal
                .map(Message::literal)
                .ok_or_else(|| D::Error::custom("message requires `literal` or `translatable`")),
        }
    }
}

/// Request to disconnect a selected player, optionally with a custom message.
///
/// This is the item type accepted by [`crate::PlayersApi::kick`]. The target
/// selector is required; omitting `message` lets Minecraft use its normal
/// disconnect text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KickPlayer {
    /// Required selector identifying the player to disconnect.
    pub player: PlayerRef,
    /// Optional literal or translatable message shown on disconnect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,
}

impl KickPlayer {
    /// Creates a kick request that lets Minecraft choose the disconnect text.
    pub fn new(player: PlayerRef) -> Self {
        Self {
            player,
            message: None,
        }
    }

    /// Creates a kick request with a literal or translatable disconnect message.
    pub fn with_message(player: PlayerRef, message: Message) -> Self {
        Self {
            player,
            message: Some(message),
        }
    }
}

/// System message delivered to all players or an explicitly selected subset.
///
/// This model is accepted by [`crate::ServerApi::system_message`]. `overlay`
/// selects action-bar rendering rather than normal chat-style rendering.
/// `receiving_players: None` means all applicable recipients; an explicit empty
/// recipient list is serialized as an empty array and is intentionally distinct
/// from omitting the field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemMessage {
    /// Literal or translatable content to deliver.
    pub message: Message,
    /// Whether Minecraft should render the message as an action-bar overlay.
    pub overlay: bool,
    /// Optional selected recipients; `None` sends to all applicable players.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receiving_players: Option<Vec<PlayerRef>>,
}

impl SystemMessage {
    /// Creates a normal chat-style system message for all applicable players.
    pub fn chat(message: Message) -> Self {
        Self {
            message,
            overlay: false,
            receiving_players: None,
        }
    }

    /// Creates an action-bar overlay message for all applicable players.
    pub fn action_bar(message: Message) -> Self {
        Self {
            message,
            overlay: true,
            receiving_players: None,
        }
    }

    /// Restricts this message to the supplied player selectors.
    ///
    /// Passing an empty iterator serializes an explicit empty recipient list;
    /// it does not restore broadcast-to-all behavior. Create a fresh
    /// `SystemMessage` without calling this method to broadcast normally.
    pub fn to(mut self, players: impl IntoIterator<Item = PlayerRef>) -> Self {
        self.receiving_players = Some(players.into_iter().collect());
        self
    }
}
