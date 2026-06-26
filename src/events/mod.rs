//! Strongly typed MCSMP notifications, raw notification access, and event streams.
//!
//! MCSMP notifications are JSON-RPC messages without request identifiers. A
//! session reader normalizes supported historical names, publishes the raw
//! method and parameter value, and then attempts to decode a corresponding
//! [`Event`]. Unknown methods and incompatible payloads remain observable as
//! [`Event::Unknown`] instead of terminating the WebSocket session.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{IpBan, Operator, PlayerRef, ServerState, TypedGameRule, UserBan};

mod decode;
mod normalize;
mod stream;

pub use stream::{EventStream, EventStreamError};

pub(crate) use decode::decode_event;
pub(crate) use normalize::normalize_notification;

/// Normalized, untyped JSON-RPC notification emitted by an MCSMP server.
///
/// `method` is the complete notification name and `params` is the optional
/// JSON parameter value exactly as received after name normalization. In
/// [`crate::CompatibilityMode::Compatible`] and
/// [`crate::CompatibilityMode::Permissive`] modes, historical
/// `notification:*` names are converted to the current
/// `minecraft:notification/*` namespace before being exposed.
///
/// Obtain raw notifications through [`crate::Client::subscribe_notifications`]
/// or through [`Event::Unknown`]. Raw access is useful for mod-defined
/// notifications and for forward compatibility with newer protocol revisions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RawNotification {
    /// Complete normalized JSON-RPC notification method name.
    ///
    /// For example, a legacy `notification:players/joined` name is exposed as
    /// `minecraft:notification/players/joined` in compatible modes.
    pub method: String,
    /// Optional JSON parameter value supplied by the notification.
    ///
    /// This is `None` when the JSON-RPC notification omitted `params`; it is
    /// otherwise retained as arbitrary `serde_json::Value`.
    pub params: Option<Value>,
}

/// Strongly typed server notification emitted through [`crate::EventStream`].
///
/// The variants map to official MCSMP notification methods. Unknown extension
/// methods, future protocol methods, capability-gated preview methods, and
/// known names with an incompatible payload are represented by [`Self::Unknown`]
/// rather than terminating the connection. The enum is `non_exhaustive`, so
/// downstream matches should include a fallback arm.
///
/// Event delivery is lossy for a subscriber that does not keep up with the
/// bounded broadcast buffer; handle [`crate::EventStreamError::Lagged`] by
/// re-querying authoritative resource state.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Event {
    /// The dedicated server completed startup (`.../server/started`).
    ServerStarted,
    /// The dedicated server began graceful shutdown (`.../server/stopping`).
    ServerStopping,
    /// World saving began (`.../server/saving`).
    ServerSaving,
    /// World saving completed (`.../server/saved`).
    ServerSaved,
    /// A server-status heartbeat was emitted (`.../server/status`).
    ServerStatus {
        /// Point-in-time server state carried by the heartbeat.
        status: ServerState,
    },
    /// A management-network activity notification was emitted.
    ///
    /// This optional event corresponds to `.../server/activity` and is only
    /// typed when the server advertises the feature through discovery.
    ServerActivity,

    /// A player connected to the server (`.../players/joined`).
    PlayerJoined {
        /// The player that joined.
        player: PlayerRef,
    },
    /// A player disconnected from the server (`.../players/left`).
    PlayerLeft {
        /// The player that left.
        player: PlayerRef,
    },

    /// A player became an operator (`.../operators/added`).
    OperatorAdded {
        /// The operator entry that was added.
        operator: Operator,
    },
    /// A player was removed from the operator list (`.../operators/removed`).
    OperatorRemoved {
        /// The operator entry that was removed.
        operator: Operator,
    },

    /// A player was added to the allowlist (`.../allowlist/added`).
    AllowlistAdded {
        /// The player that was added.
        player: PlayerRef,
    },
    /// A player was removed from the allowlist (`.../allowlist/removed`).
    AllowlistRemoved {
        /// The player that was removed.
        player: PlayerRef,
    },

    /// An IP address was added to the IP-ban list (`.../ip_bans/added`).
    IpBanAdded {
        /// The ban entry that was added.
        ban: IpBan,
    },
    /// An IP address was removed from the IP-ban list (`.../ip_bans/removed`).
    IpBanRemoved {
        /// The IP address that was unbanned.
        ip: String,
    },

    /// A player was added to the user-ban list (`.../bans/added`).
    UserBanAdded {
        /// The user ban that was added.
        ban: UserBan,
    },
    /// A player was removed from the user-ban list (`.../bans/removed`).
    UserBanRemoved {
        /// The player that was unbanned.
        player: PlayerRef,
    },

    /// A gamerule changed (`.../gamerules/updated`).
    GameRuleUpdated {
        /// The gamerule after the update.
        gamerule: TypedGameRule,
    },

    /// World-upgrade processing started.
    ///
    /// This preview event is typed only when discovery advertises world-upgrade
    /// notifications; otherwise it is surfaced as [`Self::Unknown`].
    WorldUpgradeStarted,
    /// World-upgrade progress changed.
    ///
    /// This preview event is typed only when discovery advertises world-upgrade
    /// notifications; otherwise it is surfaced as [`Self::Unknown`].
    WorldUpgradeProgress {
        /// Completed fraction in the inclusive range `0.0..=1.0`.
        progress: f64,
    },
    /// World-upgrade processing completed.
    ///
    /// This preview event is typed only when discovery advertises world-upgrade
    /// notifications; otherwise it is surfaced as [`Self::Unknown`].
    WorldUpgradeFinished,
    /// World-upgrade processing failed.
    ///
    /// This preview event is typed only when discovery advertises world-upgrade
    /// notifications; otherwise it is surfaced as [`Self::Unknown`].
    WorldUpgradeFailed {
        /// Server-supplied diagnostic reason for the failed world upgrade.
        reason: String,
    },

    /// Extension, future, malformed, or capability-gated notification.
    ///
    /// Inspect the contained [`RawNotification`] when an application wants to
    /// support a custom notification before the crate has a dedicated variant.
    Unknown(RawNotification),
}
