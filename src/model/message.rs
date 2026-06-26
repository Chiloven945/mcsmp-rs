use serde::de::Error as _;
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::player::{ensure_not_blank, ModelError};
use super::PlayerRef;

/// A display message represented as literal text or a Minecraft translation
/// key with parameters.
///
/// If a payload contains both a translation key and literal text, MCSMP gives
/// the translation form precedence. Deserializing such a payload therefore
/// yields [`Message::Translatable`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Message {
    /// Literal text that Minecraft displays without translation-key lookup.
    Literal(String),
    /// A Minecraft translation key and the values substituted into it.
    Translatable {
        /// The non-blank translation key.
        key: String,
        /// Values supplied to the translated message.
        params: Vec<String>,
    },
}

impl Message {
    /// Creates a literal message.
    pub fn literal(text: impl Into<String>) -> Self {
        Self::Literal(text.into())
    }

    /// Creates a translation-key message.
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

    /// Returns literal text when this is a literal message.
    pub fn literal_text(&self) -> Option<&str> {
        match self {
            Self::Literal(text) => Some(text),
            Self::Translatable { .. } => None,
        }
    }

    /// Returns the translation key when this is a translatable message.
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

/// A request to disconnect a selected player, optionally with a custom message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KickPlayer {
    /// The player who should be disconnected.
    pub player: PlayerRef,
    /// An optional message shown when the player is disconnected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,
}

impl KickPlayer {
    /// Creates a kick request without a custom message.
    pub fn new(player: PlayerRef) -> Self {
        Self {
            player,
            message: None,
        }
    }

    /// Creates a kick request with a custom disconnect message.
    pub fn with_message(player: PlayerRef, message: Message) -> Self {
        Self {
            player,
            message: Some(message),
        }
    }
}

/// A system message sent to all players or a selected group of players.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemMessage {
    /// The message content to send.
    pub message: Message,
    /// Whether the message is shown as an action-bar overlay.
    pub overlay: bool,
    /// Optional selected recipients; `None` sends to all applicable players.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receiving_players: Option<Vec<PlayerRef>>,
}

impl SystemMessage {
    /// Creates a normal chat/system message for all players.
    pub fn chat(message: Message) -> Self {
        Self {
            message,
            overlay: false,
            receiving_players: None,
        }
    }

    /// Creates an action-bar overlay message for all players.
    pub fn action_bar(message: Message) -> Self {
        Self {
            message,
            overlay: true,
            receiving_players: None,
        }
    }

    /// Targets this message to the provided player selectors.
    pub fn to(mut self, players: impl IntoIterator<Item = PlayerRef>) -> Self {
        self.receiving_players = Some(players.into_iter().collect());
        self
    }
}
