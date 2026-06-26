use serde::{Deserialize, Serialize};

use super::player::{ensure_not_blank, ModelError};
use super::PlayerRef;

/// A Minecraft game version returned by server status.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinecraftVersion {
    /// The human-readable game version or snapshot name.
    pub name: String,
    /// The Minecraft network protocol number.
    pub protocol: i32,
}

impl MinecraftVersion {
    /// Creates a version descriptor with a non-blank display name.
    pub fn new(name: impl Into<String>, protocol: i32) -> Result<Self, ModelError> {
        let name = name.into();
        ensure_not_blank("name", &name)?;
        Ok(Self { name, protocol })
    }
}

/// A server operator entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operator {
    /// The player who is or should become an operator.
    pub player: PlayerRef,
    /// An optional command permission level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission_level: Option<i32>,
    /// Whether this operator may bypass the player limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bypasses_player_limit: Option<bool>,
}

impl Operator {
    /// Creates an operator entry that lets the server apply its default options.
    pub fn new(player: PlayerRef) -> Self {
        Self {
            player,
            permission_level: None,
            bypasses_player_limit: None,
        }
    }

    /// Creates an operator entry with an explicitly validated permission level.
    pub fn with_permission_level(
        player: PlayerRef,
        permission_level: i32,
    ) -> Result<Self, ModelError> {
        validate_permission_level(permission_level)?;
        Ok(Self {
            player,
            permission_level: Some(permission_level),
            bypasses_player_limit: None,
        })
    }

    /// Creates an operator entry with explicit permission and limit-bypass
    /// options.
    pub fn with_options(
        player: PlayerRef,
        permission_level: i32,
        bypasses_player_limit: bool,
    ) -> Result<Self, ModelError> {
        validate_permission_level(permission_level)?;
        Ok(Self {
            player,
            permission_level: Some(permission_level),
            bypasses_player_limit: Some(bypasses_player_limit),
        })
    }
}

fn validate_permission_level(value: i32) -> Result<(), ModelError> {
    if (0..=4).contains(&value) {
        Ok(())
    } else {
        Err(ModelError::InvalidOperatorPermissionLevel { value })
    }
}

/// A snapshot of a Minecraft dedicated server's lifecycle state and online
/// players.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerState {
    /// Whether the dedicated server has completed startup.
    pub started: bool,
    /// Players connected when the status response was generated.
    pub players: Vec<PlayerRef>,
    /// The Minecraft game version, not the MCSMP protocol version.
    pub version: MinecraftVersion,
}

impl ServerState {
    /// Returns the number of players in this status snapshot.
    pub fn online_player_count(&self) -> usize {
        self.players.len()
    }
}
