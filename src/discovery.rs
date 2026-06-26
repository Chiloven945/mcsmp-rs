//! Capability discovery and protocol-version support.
//!
//! MCSMP servers expose `rpc.discover`, which returns a JSON schema describing
//! the methods and notifications supported by the running server. The schema is
//! retained verbatim because it may contain server-specific extensions that are
//! not represented by this crate's typed APIs.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

use serde_json::Value;

use crate::Error;

/// Numeric semantic version of the MCSMP protocol.
///
/// This version is distinct from Minecraft's game/network version. Pre-release
/// and build suffixes are accepted while parsing but are not retained because
/// MCSMP feature gates currently use only numeric components.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProtocolVersion {
    /// Major protocol version component.
    pub major: u64,
    /// Minor protocol version component.
    pub minor: u64,
    /// Patch protocol version component.
    pub patch: u64,
}

impl ProtocolVersion {
    /// The first known version of MCSMP.
    pub const V1_0_0: Self = Self::new(1, 0, 0);
    /// The version that added `server/activity` notifications.
    pub const V1_1_0: Self = Self::new(1, 1, 0);
    /// The version that introduced typed gamerule scalars.
    pub const V2_0_0: Self = Self::new(2, 0, 0);
    /// The version that made discovery usable before server startup completes.
    pub const V3_0_0: Self = Self::new(3, 0, 0);
    /// The version that introduced world-upgrade notifications.
    pub const V3_1_0: Self = Self::new(3, 1, 0);

    /// Creates a numeric protocol version.
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Returns whether this version is at least `minimum`.
    pub const fn is_at_least(self, minimum: Self) -> bool {
        self.major > minimum.major
            || (self.major == minimum.major && self.minor > minimum.minor)
            || (self.major == minimum.major
                && self.minor == minimum.minor
                && self.patch >= minimum.patch)
    }

    /// Parses a semantic version, ignoring an optional pre-release or build
    /// suffix. Returns `None` for an invalid version string.
    pub fn parse(value: &str) -> Option<Self> {
        value.parse().ok()
    }
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// A parse error returned by [`ProtocolVersion::from_str`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolVersionParseError;

impl fmt::Display for ProtocolVersionParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("expected a semantic version in major.minor.patch form")
    }
}

impl std::error::Error for ProtocolVersionParseError {}

impl FromStr for ProtocolVersion {
    type Err = ProtocolVersionParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let trimmed = value.trim();
        let numeric = trimmed
            .split(|character| character == '-' || character == '+')
            .next()
            .unwrap_or(trimmed);
        let mut parts = numeric.split('.');
        let major = parts
            .next()
            .and_then(|part| part.parse().ok())
            .ok_or(ProtocolVersionParseError)?;
        let minor = parts
            .next()
            .and_then(|part| part.parse().ok())
            .ok_or(ProtocolVersionParseError)?;
        let patch = parts
            .next()
            .and_then(|part| part.parse().ok())
            .ok_or(ProtocolVersionParseError)?;
        if parts.next().is_some() {
            return Err(ProtocolVersionParseError);
        }
        Ok(Self::new(major, minor, patch))
    }
}

/// Optional behavior inferred from the advertised MCSMP version and schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Feature {
    /// The management endpoint requires a client secret.
    Authentication,
    /// TLS is enabled by default for the endpoint.
    TlsByDefault,
    /// Notifications use the `minecraft:notification/` prefix.
    MinecraftNotificationPrefix,
    /// `minecraft:notification/server/activity` is available.
    ServerActivityNotification,
    /// The server supports browser-oriented origin allowlisting.
    OriginAllowlist,
    /// Gamerule values use JSON booleans and integers rather than only strings.
    TypedGameruleValue,
    /// Discovery and server status are available before full server startup.
    PreStartDiscovery,
    /// World-upgrade lifecycle notifications are available.
    WorldUpgradeNotifications,
}

/// A capability snapshot returned by [`crate::Client::discover`].
#[derive(Clone, Debug, PartialEq)]
pub struct Capabilities {
    /// Advertised MCSMP version when the schema contained a parsable value.
    pub protocol_version: Option<ProtocolVersion>,
    /// Full JSON-RPC method names advertised by the server.
    pub methods: BTreeSet<String>,
    /// Full notification method names advertised by the server.
    pub notifications: BTreeSet<String>,
    /// Stable feature flags inferred from version and named capabilities.
    pub features: BTreeSet<Feature>,
    /// Complete unmodified value returned by `rpc.discover`.
    pub raw_schema: Value,
    /// Per-method schema fragments when the discovery shape exposed them.
    pub method_schemas: BTreeMap<String, Value>,
    /// Per-notification schema fragments when the discovery shape exposed them.
    pub notification_schemas: BTreeMap<String, Value>,
}

impl Capabilities {
    /// Returns whether a full JSON-RPC method name was advertised.
    pub fn supports_method(&self, method: &str) -> bool {
        self.methods.contains(method)
    }

    /// Returns whether a full notification method name was advertised.
    pub fn supports_notification(&self, method: &str) -> bool {
        self.notifications.contains(method)
    }

    /// Returns whether this snapshot implies `feature`.
    pub fn supports_feature(&self, feature: Feature) -> bool {
        self.features.contains(&feature)
    }

    /// Returns an error when this snapshot does not imply `feature`.
    pub fn require_feature(&self, feature: Feature) -> Result<(), Error> {
        if self.supports_feature(feature) {
            Ok(())
        } else {
            Err(Error::UnsupportedFeature(feature))
        }
    }

    /// Creates a capability summary from an arbitrary discovery response.
    ///
    /// The parser intentionally accepts both arrays of `{ "name": ... }`
    /// records and object maps keyed by method/notification names. Unknown
    /// fields are retained in [`Capabilities::raw_schema`] instead of causing a
    /// discovery failure.
    pub fn from_schema(raw_schema: Value) -> Self {
        let protocol_version = extract_protocol_version(&raw_schema);
        let (methods, method_schemas) = extract_entries(&raw_schema, "methods");
        let (notifications, notification_schemas) = extract_entries(&raw_schema, "notifications");
        let features = infer_features(protocol_version, &methods, &notifications);

        Self {
            protocol_version,
            methods,
            notifications,
            features,
            raw_schema,
            method_schemas,
            notification_schemas,
        }
    }
}

fn extract_protocol_version(schema: &Value) -> Option<ProtocolVersion> {
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

fn extract_entries(schema: &Value, key: &str) -> (BTreeSet<String>, BTreeMap<String, Value>) {
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

fn infer_features(
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn parses_semver_with_suffix() {
        assert_eq!(
            "3.1.0-snapshot.1".parse::<ProtocolVersion>().unwrap(),
            ProtocolVersion::V3_1_0
        );
        assert!("3.1".parse::<ProtocolVersion>().is_err());
    }

    #[test]
    fn accepts_array_and_map_discovery_shapes() {
        let capabilities = Capabilities::from_schema(json!({
            "protocolVersion": "3.1.0",
            "methods": [{"name": "rpc.discover"}, {"name": "minecraft:server/status"}],
            "notifications": {
                "minecraft:notification/server/activity": {"params": {}},
                "minecraft:notification/world/upgrade_started": {}
            }
        }));

        assert!(capabilities.supports_method("rpc.discover"));
        assert!(capabilities.supports_notification("minecraft:notification/server/activity"));
        assert!(capabilities.supports_feature(Feature::TypedGameruleValue));
        assert!(capabilities.supports_feature(Feature::WorldUpgradeNotifications));
        assert_eq!(capabilities.protocol_version, Some(ProtocolVersion::V3_1_0));
    }
}
