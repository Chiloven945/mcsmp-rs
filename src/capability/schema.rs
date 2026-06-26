//! Tolerant parsing for `rpc.discover` response schemas.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::{Feature, ProtocolVersion};

pub(crate) fn extract_protocol_version(schema: &Value) -> Option<ProtocolVersion> {
    let object = schema.as_object()?;
    for key in [
        "protocolVersion",
        "protocol_version",
        "mcsmpVersion",
        "mcsmp_version",
        "version",
    ] {
        let Some(value) = object.get(key) else {
            continue;
        };
        if let Some(version) = value.as_str().and_then(ProtocolVersion::parse) {
            return Some(version);
        }
        if let Some(version) = value
            .as_object()
            .and_then(|value| value.get("version").or_else(|| value.get("name")))
            .and_then(Value::as_str)
            .and_then(ProtocolVersion::parse)
        {
            return Some(version);
        }
    }
    None
}

pub(crate) fn extract_entries(
    schema: &Value,
    key: &str,
) -> (BTreeSet<String>, BTreeMap<String, Value>) {
    let mut names = BTreeSet::new();
    let mut fragments = BTreeMap::new();
    let Some(value) = schema.as_object().and_then(|object| object.get(key)) else {
        return (names, fragments);
    };

    match value {
        Value::Array(entries) => {
            for entry in entries.iter() {
                match entry {
                    Value::String(name) if !name.trim().is_empty() => {
                        names.insert(name.clone());
                    }
                    Value::Object(object) => {
                        let name = object
                            .get("name")
                            .or_else(|| object.get("method"))
                            .or_else(|| object.get("id"))
                            .and_then(Value::as_str)
                            .filter(|name| !name.trim().is_empty());
                        if let Some(name) = name {
                            names.insert(name.to_owned());
                            fragments.insert(name.to_owned(), entry.clone());
                        }
                    }
                    _ => {}
                }
            }
        }
        Value::Object(entries) => {
            if let Some(name) = entries
                .get("name")
                .or_else(|| entries.get("method"))
                .and_then(Value::as_str)
                .filter(|name| !name.trim().is_empty())
            {
                names.insert(name.to_owned());
                fragments.insert(name.to_owned(), value.clone());
            } else {
                for (name, fragment) in entries {
                    if !name.trim().is_empty() {
                        names.insert(name.clone());
                        fragments.insert(name.clone(), fragment.clone());
                    }
                }
            }
        }
        _ => {}
    }

    (names, fragments)
}

pub(crate) fn infer_features(
    version: Option<ProtocolVersion>,
    methods: &BTreeSet<String>,
    notifications: &BTreeSet<String>,
) -> BTreeSet<Feature> {
    let mut features = BTreeSet::new();
    let at_least = |minimum| version.is_some_and(|version| version.is_at_least(minimum));

    if at_least(ProtocolVersion::V1_0_0) {
        features.insert(Feature::Authentication);
        features.insert(Feature::TlsByDefault);
    }
    if at_least(ProtocolVersion::V1_1_0) {
        features.insert(Feature::ServerActivityNotification);
        features.insert(Feature::OriginAllowlist);
    }
    if at_least(ProtocolVersion::V2_0_0) {
        features.insert(Feature::TypedGameruleValue);
    }
    if at_least(ProtocolVersion::V3_0_0) {
        features.insert(Feature::PreStartDiscovery);
    }
    if at_least(ProtocolVersion::V3_1_0) {
        features.insert(Feature::WorldUpgradeNotifications);
    }

    if notifications
        .iter()
        .any(|name| name.starts_with("minecraft:notification/"))
    {
        features.insert(Feature::MinecraftNotificationPrefix);
    }
    if notifications.contains("minecraft:notification/server/activity")
        || notifications.contains("notification:server/activity")
    {
        features.insert(Feature::ServerActivityNotification);
    }
    if notifications
        .iter()
        .any(|name| name.starts_with("minecraft:notification/world/upgrade_"))
    {
        features.insert(Feature::WorldUpgradeNotifications);
    }
    if methods.contains("rpc.discover")
        && methods.contains("minecraft:server/status")
        && at_least(ProtocolVersion::V3_0_0)
    {
        features.insert(Feature::PreStartDiscovery);
    }

    features
}
