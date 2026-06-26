//! Strongly typed MCSMP notification models and event streams.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{IpBan, Operator, PlayerRef, ServerState, TypedGameRule, UserBan};

mod decode;
mod normalize;
mod stream;

pub use stream::{EventStream, EventStreamError};

pub(crate) use decode::decode_event;
pub(crate) use normalize::normalize_notification;

/// An untyped JSON-RPC notification emitted by an MCSMP server.
///
/// In [`CompatibilityMode::Compatible`] and [`CompatibilityMode::Permissive`]
/// modes, historical `notification:*` names are normalized to the current
/// `minecraft:notification/*` namespace before being exposed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RawNotification {
    /// Fully-qualified, normalized JSON-RPC notification method.
    pub method: String,
    /// Optional JSON-RPC notification parameters.
    pub params: Option<Value>,
}

/// A strongly typed server notification.
///
/// Unknown notification methods and known methods with an incompatible payload
/// are represented by [`Event::Unknown`] rather than terminating the transport
/// connection. This keeps mod-defined extension notifications forward
/// compatible.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Event {
    /// The dedicated server completed startup.
    ServerStarted,
    /// The dedicated server began shutting down.
    ServerStopping,
    /// World saving began.
    ServerSaving,
    /// World saving completed.
    ServerSaved,
    /// A server-status heartbeat was emitted.
    ServerStatus {
        /// Current server state reported by the heartbeat.
        status: ServerState,
    },
    /// A management connection was initialized.
    ServerActivity,

    /// A player connected to the server.
    PlayerJoined {
        /// The player that joined.
        player: PlayerRef,
    },
    /// A player disconnected from the server.
    PlayerLeft {
        /// The player that left.
        player: PlayerRef,
    },

    /// A player became an operator.
    OperatorAdded {
        /// The operator entry that was added.
        operator: Operator,
    },
    /// A player was removed from the operator list.
    OperatorRemoved {
        /// The operator entry that was removed.
        operator: Operator,
    },

    /// A player was added to the allowlist.
    AllowlistAdded {
        /// The player that was added.
        player: PlayerRef,
    },
    /// A player was removed from the allowlist.
    AllowlistRemoved {
        /// The player that was removed.
        player: PlayerRef,
    },

    /// An IP address was banned.
    IpBanAdded {
        /// The ban entry that was added.
        ban: IpBan,
    },
    /// An IP address was removed from the ban list.
    IpBanRemoved {
        /// The IP address that was unbanned.
        ip: String,
    },

    /// A player was added to the user-ban list.
    UserBanAdded {
        /// The user ban that was added.
        ban: UserBan,
    },
    /// A player was removed from the user-ban list.
    UserBanRemoved {
        /// The player that was unbanned.
        player: PlayerRef,
    },

    /// A gamerule changed.
    GameRuleUpdated {
        /// The gamerule after the update.
        gamerule: TypedGameRule,
    },

    /// World upgrade processing started.
    WorldUpgradeStarted,
    /// World upgrade progress changed.
    WorldUpgradeProgress {
        /// Completed fraction in the inclusive range 0.0 through 1.0.
        progress: f64,
    },
    /// World upgrade processing completed.
    WorldUpgradeFinished,
    /// World upgrade processing failed.
    WorldUpgradeFailed {
        /// Server-supplied failure reason.
        reason: String,
    },

    /// An extension, future, malformed, or capability-gated notification.
    Unknown(RawNotification),
}
