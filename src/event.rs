//! Strongly typed MCSMP notifications and event-stream support.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::sync::broadcast;

use crate::{
    Capabilities, CompatibilityMode, Feature, IpBan, Operator, PlayerRef, ServerState,
    TypedGameRule, UserBan,
};

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

/// Backwards-compatible alias for [`RawNotification`].
pub type Notification = RawNotification;

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

/// A stream of [`Event`] values produced by [`crate::Client::subscribe`].
pub struct EventStream {
    inner: Pin<Box<dyn Stream<Item = Result<Event, EventStreamError>> + Send>>,
}

impl std::fmt::Debug for EventStream {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EventStream")
            .finish_non_exhaustive()
    }
}

impl EventStream {
    pub(crate) fn new(
        receiver: broadcast::Receiver<Event>,
        shutdown: tokio::sync::watch::Receiver<bool>,
    ) -> Self {
        struct State {
            receiver: broadcast::Receiver<Event>,
            shutdown: tokio::sync::watch::Receiver<bool>,
        }

        let stream =
            futures_util::stream::unfold(State { receiver, shutdown }, |mut state| async move {
                if *state.shutdown.borrow() {
                    return None;
                }
                let item = tokio::select! {
                    changed = state.shutdown.changed() => {
                        if changed.is_err() || *state.shutdown.borrow() {
                            return None;
                        }
                        return None;
                    }
                    received = state.receiver.recv() => match received {
                        Ok(event) => Ok(event),
                        Err(broadcast::error::RecvError::Lagged(dropped)) => {
                            Err(EventStreamError::Lagged { dropped })
                        }
                        Err(broadcast::error::RecvError::Closed) => return None,
                    },
                };
                Some((item, state))
            });
        Self {
            inner: Box::pin(stream),
        }
    }

    /// Waits for the next event.
    ///
    /// [`EventStreamError::Lagged`] means this subscriber missed events and
    /// should re-synchronize state through the appropriate query API. A closed
    /// stream returns [`EventStreamError::Closed`].
    pub async fn recv(&mut self) -> Result<Event, EventStreamError> {
        match self.next().await {
            Some(Ok(event)) => Ok(event),
            Some(Err(error)) => Err(error),
            None => Err(EventStreamError::Closed),
        }
    }
}

impl Stream for EventStream {
    type Item = Result<Event, EventStreamError>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().inner.as_mut().poll_next(context)
    }
}

/// A recoverable error emitted while consuming an [`EventStream`].
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum EventStreamError {
    /// The subscriber did not consume events quickly enough for the bounded
    /// broadcast buffer.
    #[error("event subscriber lagged behind and missed {dropped} events")]
    Lagged {
        /// Number of events discarded for this subscriber.
        dropped: u64,
    },
    /// The client closed its event broadcaster.
    #[error("event stream is closed")]
    Closed,
}

pub(crate) fn normalize_notification(
    raw: RawNotification,
    compatibility_mode: CompatibilityMode,
) -> Result<RawNotification, String> {
    if raw.method.starts_with("minecraft:notification/") {
        return Ok(raw);
    }

    let Some(suffix) = raw.method.strip_prefix("notification:") else {
        return Ok(raw);
    };

    if compatibility_mode == CompatibilityMode::Strict {
        return Err(format!(
            "legacy notification prefix is not permitted in strict mode: `{}`",
            raw.method
        ));
    }

    Ok(RawNotification {
        method: format!("minecraft:notification/{suffix}"),
        params: raw.params,
    })
}

pub(crate) fn decode_event(raw: RawNotification, capabilities: Option<&Capabilities>) -> Event {
    let world_upgrade_supported = capabilities.is_some_and(|capabilities| {
        capabilities.supports_feature(Feature::WorldUpgradeNotifications)
            || capabilities.supports_notification("minecraft:notification/world/upgrade_started")
    });

    let event = match raw.method.as_str() {
        "minecraft:notification/server/started" => Some(Event::ServerStarted),
        "minecraft:notification/server/stopping" => Some(Event::ServerStopping),
        "minecraft:notification/server/saving" => Some(Event::ServerSaving),
        "minecraft:notification/server/saved" => Some(Event::ServerSaved),
        "minecraft:notification/server/status" => {
            decode_named(&raw, &["status"]).map(|status| Event::ServerStatus { status })
        }
        "minecraft:notification/server/activity" => Some(Event::ServerActivity),

        "minecraft:notification/players/joined" => {
            decode_named(&raw, &["player"]).map(|player| Event::PlayerJoined { player })
        }
        "minecraft:notification/players/left" => {
            decode_named(&raw, &["player"]).map(|player| Event::PlayerLeft { player })
        }

        "minecraft:notification/operators/added" => decode_named(&raw, &["player", "operator"])
            .map(|operator| Event::OperatorAdded { operator }),
        "minecraft:notification/operators/removed" => decode_named(&raw, &["player", "operator"])
            .map(|operator| Event::OperatorRemoved { operator }),

        "minecraft:notification/allowlist/added" => {
            decode_named(&raw, &["player"]).map(|player| Event::AllowlistAdded { player })
        }
        "minecraft:notification/allowlist/removed" => {
            decode_named(&raw, &["player"]).map(|player| Event::AllowlistRemoved { player })
        }

        "minecraft:notification/ip_bans/added" => {
            decode_named(&raw, &["ban", "player"]).map(|ban| Event::IpBanAdded { ban })
        }
        "minecraft:notification/ip_bans/removed" => {
            decode_named(&raw, &["ip", "player"]).map(|ip| Event::IpBanRemoved { ip })
        }

        "minecraft:notification/bans/added" => {
            decode_named(&raw, &["ban", "player"]).map(|ban| Event::UserBanAdded { ban })
        }
        "minecraft:notification/bans/removed" => {
            decode_named(&raw, &["player"]).map(|player| Event::UserBanRemoved { player })
        }

        "minecraft:notification/gamerules/updated" => {
            decode_named(&raw, &["gamerule"]).map(|gamerule| Event::GameRuleUpdated { gamerule })
        }

        "minecraft:notification/world/upgrade_started" if world_upgrade_supported => {
            Some(Event::WorldUpgradeStarted)
        }
        "minecraft:notification/world/upgrade_progress" if world_upgrade_supported => {
            decode_named(&raw, &["progress"]).and_then(|progress: f64| {
                (0.0..=1.0)
                    .contains(&progress)
                    .then_some(Event::WorldUpgradeProgress { progress })
            })
        }
        "minecraft:notification/world/upgrade_finished" if world_upgrade_supported => {
            Some(Event::WorldUpgradeFinished)
        }
        "minecraft:notification/world/upgrade_failed" if world_upgrade_supported => {
            decode_named(&raw, &["reason"]).map(|reason| Event::WorldUpgradeFailed { reason })
        }
        _ => None,
    };

    event.unwrap_or(Event::Unknown(raw))
}

fn decode_named<T>(raw: &RawNotification, names: &[&str]) -> Option<T>
where
    T: serde::de::DeserializeOwned,
{
    let object = raw.params.as_ref()?.as_object()?;
    names.iter().find_map(|name| {
        object
            .get(*name)
            .and_then(|value| serde_json::from_value(value.clone()).ok())
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::CompatibilityMode;

    #[test]
    fn normalizes_legacy_prefix_unless_strict() {
        let raw = RawNotification {
            method: "notification:players/joined".into(),
            params: Some(json!({"player": {"name": "Alex"}})),
        };
        let normalized =
            normalize_notification(raw.clone(), CompatibilityMode::Compatible).unwrap();
        assert_eq!(normalized.method, "minecraft:notification/players/joined");
        assert!(normalize_notification(raw, CompatibilityMode::Strict).is_err());
    }

    #[test]
    fn malformed_known_payload_falls_back_to_unknown() {
        let event = decode_event(
            RawNotification {
                method: "minecraft:notification/players/joined".into(),
                params: Some(json!({"player": 5})),
            },
            None,
        );
        assert!(matches!(event, Event::Unknown(_)));
    }

    #[test]
    fn world_upgrade_requires_advertised_capability() {
        let raw = RawNotification {
            method: "minecraft:notification/world/upgrade_started".into(),
            params: None,
        };
        assert!(matches!(decode_event(raw.clone(), None), Event::Unknown(_)));

        let capabilities = Capabilities::from_schema(json!({
            "protocolVersion": "3.1.0",
            "notifications": ["minecraft:notification/world/upgrade_started"]
        }));
        assert!(matches!(
            decode_event(raw, Some(&capabilities)),
            Event::WorldUpgradeStarted
        ));
    }
}
