use serde::{Deserialize, Serialize};

use super::PlayerRef;
use super::player::{ModelError, ensure_not_blank};

/// Minecraft game version reported by `minecraft:server/status`.
///
/// This identifies the running Minecraft server, not the MCSMP management
/// protocol. Use [`crate::ProtocolVersion`] from capability discovery when an
/// application needs to gate behavior on the management protocol version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinecraftVersion {
    /// Human-readable Minecraft release or snapshot name, such as `1.21.9`.
    pub name: String,
    /// Minecraft game network protocol number, distinct from MCSMP versioning.
    pub protocol: i32,
}

impl MinecraftVersion {
    /// Creates a game-version descriptor with a non-blank display name.
    ///
    /// The network protocol number is preserved exactly, including unknown or
    /// future values. Empty and whitespace-only names return
    /// [`ModelError::BlankField`].
    pub fn new(name: impl Into<String>, protocol: i32) -> Result<Self, ModelError> {
        let name = name.into();
        ensure_not_blank("name", &name)?;
        Ok(Self { name, protocol })
    }
}

/// Minecraft server operator entry.
///
/// The optional fields are omitted from serialization when `None`, allowing
/// the server to apply its default operator options. When a permission level is
/// supplied through the constructors, it is validated against Minecraft's
/// supported `0..=4` range.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operator {
    /// Player selector that identifies the operator account.
    pub player: PlayerRef,
    /// Optional operator command permission level in the server's `0..=4` range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission_level: Option<i32>,
    /// Optional flag granting this operator player-limit bypass permission.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bypasses_player_limit: Option<bool>,
}

impl Operator {
    /// Creates an operator entry that leaves optional settings to server defaults.
    pub fn new(player: PlayerRef) -> Self {
        Self {
            player,
            permission_level: None,
            bypasses_player_limit: None,
        }
    }

    /// Creates an operator entry with an explicitly validated permission level.
    ///
    /// Returns [`ModelError::InvalidOperatorPermissionLevel`] unless
    /// `permission_level` is in `0..=4`.
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

    /// Creates an operator entry with explicit permission and player-limit bypass.
    ///
    /// Returns [`ModelError::InvalidOperatorPermissionLevel`] unless
    /// `permission_level` is in `0..=4`.
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

/// Point-in-time dedicated-server lifecycle and player snapshot.
///
/// This model is returned by `ServerApi::status` and embedded in status
/// heartbeat events. It is not a live view: query again or consume
/// [`crate::Event::ServerStatus`] when an application needs newer information.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerState {
    /// Whether the dedicated server has completed its startup sequence.
    pub started: bool,
    /// Players connected when this snapshot was generated.
    pub players: Vec<PlayerRef>,
    /// Running Minecraft game version, not the MCSMP protocol version.
    pub version: MinecraftVersion,
}

impl ServerState {
    /// Returns the number of players in this point-in-time snapshot.
    ///
    /// This is equivalent to `self.players.len()` and does not query the
    /// server.
    pub fn online_player_count(&self) -> usize {
        self.players.len()
    }
}
