//! Conversion from raw notification payloads into typed events.

use crate::{Capabilities, Feature};

use super::{Event, RawNotification};

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
    use super::*;
    use serde_json::json;

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
        let capabilities = Capabilities::from_schema(
            json!({ "protocolVersion": "3.1.0", "notifications": ["minecraft:notification/world/upgrade_started"] }),
        );
        assert!(matches!(
            decode_event(raw, Some(&capabilities)),
            Event::WorldUpgradeStarted
        ));
    }
}
