//! Notification-name normalization for supported protocol generations.

use crate::CompatibilityMode;

use super::RawNotification;

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
}
