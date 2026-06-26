//! Capability discovery, protocol versions, and invocation policy.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

use serde_json::Value;

use crate::Error;

mod discover;
mod schema;

pub(crate) use discover::discover_capabilities;

/// Controls how calls and historical protocol forms are handled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum CompatibilityMode {
    /// Require discovery before invoking methods and reject unadvertised methods.
    Strict,
    /// Allow known historical wire forms and extension methods.
    #[default]
    Compatible,
    /// Do not preflight calls using discovery results.
    Permissive,
}

/// Numeric semantic version of the MCSMP protocol.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProtocolVersion {
    /// Major protocol-version component.
    pub major: u64,
    /// Minor protocol-version component.
    pub minor: u64,
    /// Patch protocol-version component.
    pub patch: u64,
}

impl ProtocolVersion {
    /// The first known MCSMP protocol version.
    pub const V1_0_0: Self = Self::new(1, 0, 0);
    /// Version adding the server-activity notification.
    pub const V1_1_0: Self = Self::new(1, 1, 0);
    /// Version adding native boolean and integer gamerule values.
    pub const V2_0_0: Self = Self::new(2, 0, 0);
    /// Version enabling discovery before normal server startup completes.
    pub const V3_0_0: Self = Self::new(3, 0, 0);
    /// Version adding world-upgrade notifications.
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

    /// Parses a semantic version while ignoring an optional suffix.
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

/// Optional behavior inferred from the advertised MCSMP schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Feature {
    /// The endpoint requires a client secret.
    Authentication,
    /// TLS is enabled by default for the endpoint.
    TlsByDefault,
    /// Notifications use the `minecraft:notification/` prefix.
    MinecraftNotificationPrefix,
    /// The server-activity notification is available.
    ServerActivityNotification,
    /// Browser-oriented origin allowlisting is available.
    OriginAllowlist,
    /// Gamerule values use native JSON booleans and integers.
    TypedGameruleValue,
    /// Discovery and status work before full server startup.
    PreStartDiscovery,
    /// World-upgrade lifecycle notifications are available.
    WorldUpgradeNotifications,
}

/// A capability snapshot returned by `rpc.discover`.
#[derive(Clone, Debug, PartialEq)]
pub struct Capabilities {
    /// Advertised MCSMP version when a parseable version was present.
    pub protocol_version: Option<ProtocolVersion>,
    /// Full JSON-RPC method names advertised by the server.
    pub methods: BTreeSet<String>,
    /// Full notification names advertised by the server.
    pub notifications: BTreeSet<String>,
    /// Stable feature flags inferred from version and named capabilities.
    pub features: BTreeSet<Feature>,
    /// Unmodified value returned by `rpc.discover`.
    pub raw_schema: Value,
    /// Per-method schema fragments when available.
    pub method_schemas: BTreeMap<String, Value>,
    /// Per-notification schema fragments when available.
    pub notification_schemas: BTreeMap<String, Value>,
}

impl Capabilities {
    /// Returns whether `method` was advertised.
    pub fn supports_method(&self, method: &str) -> bool {
        self.methods.contains(method)
    }
    /// Returns whether `method` notification was advertised.
    pub fn supports_notification(&self, method: &str) -> bool {
        self.notifications.contains(method)
    }
    /// Returns whether the snapshot implies `feature`.
    pub fn supports_feature(&self, feature: Feature) -> bool {
        self.features.contains(&feature)
    }
    /// Returns an error when `feature` is unavailable.
    pub fn require_feature(&self, feature: Feature) -> Result<(), Error> {
        if self.supports_feature(feature) {
            Ok(())
        } else {
            Err(Error::UnsupportedFeature(feature))
        }
    }

    /// Builds a capability summary from any supported discovery-schema shape.
    pub fn from_schema(raw_schema: Value) -> Self {
        let protocol_version = schema::extract_protocol_version(&raw_schema);
        let (methods, method_schemas) = schema::extract_entries(&raw_schema, "methods");
        let (notifications, notification_schemas) =
            schema::extract_entries(&raw_schema, "notifications");
        let features = schema::infer_features(protocol_version, &methods, &notifications);
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
    }
}
