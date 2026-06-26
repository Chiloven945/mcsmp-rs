//! Capability discovery, protocol versions, inferred features, and call policy.
//!
//! MCSMP evolves independently from Minecraft game versions. This module keeps
//! that distinction explicit: [`ProtocolVersion`] describes the management
//! protocol advertised through `rpc.discover`, while
//! [`crate::MinecraftVersion`] describes the game server reported by
//! `minecraft:server/status`.
//!
//! Applications can call [`crate::Client::discover`] to obtain [`Capabilities`]
//! and then make behavior conditional on advertised methods, notifications, or
//! inferred [`Feature`] values.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

use serde_json::Value;

use crate::Error;

mod discover;
mod schema;

pub(crate) use discover::discover_capabilities;

/// Controls how discovery information and historical wire forms are handled.
///
/// This setting is chosen by [`crate::ClientBuilder::compatibility_mode`] and
/// is immutable for the resulting client. It affects both outgoing method
/// preflight checks and notification-name normalization.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum CompatibilityMode {
    /// Require discovery before ordinary calls and reject unadvertised methods.
    ///
    /// In this mode, calling a typed or raw method before
    /// [`crate::Client::discover`] succeeds returns
    /// [`crate::Error::DiscoveryRequired`]. After discovery, a method absent
    /// from the advertised schema returns [`crate::Error::UnsupportedMethod`]
    /// without being sent. Historical `notification:*` names are not
    /// normalized.
    Strict,
    /// Accept supported historical forms while allowing extension methods.
    ///
    /// This is the default. It accepts legacy notification prefixes and legacy
    /// gamerule strings where supported by the model layer. Discovery is
    /// optional and does not block calls to unknown extension methods.
    #[default]
    Compatible,
    /// Do not preflight outgoing calls using discovery results.
    ///
    /// This mode is appropriate for protocol exploration and servers with
    /// custom schemas. It still parses recognized historical forms, but it
    /// deliberately lets the remote peer decide whether a method exists.
    Permissive,
}

/// Numeric semantic version of the MCSMP management protocol.
///
/// This is not the Minecraft Java network protocol number and not the
/// `minecraft-v1` WebSocket subprotocol token. It is parsed from discovery
/// data when available and contains only numeric `major.minor.patch`
/// components.
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
    /// Initial known MCSMP `1.0.0` generation.
    ///
    /// This is a historical compatibility marker; use discovery for an exact
    /// server capability decision.
    pub const V1_0_0: Self = Self::new(1, 0, 0);
    /// MCSMP `1.1.0`, associated with server-activity notifications.
    pub const V1_1_0: Self = Self::new(1, 1, 0);
    /// MCSMP `2.0.0`, associated with native boolean/integer gamerule values.
    pub const V2_0_0: Self = Self::new(2, 0, 0);
    /// MCSMP `3.0.0`, associated with pre-start discovery and status support.
    pub const V3_0_0: Self = Self::new(3, 0, 0);
    /// MCSMP `3.1.0`, associated with world-upgrade notifications.
    ///
    /// Treat this as a feature-inference marker only. Check discovery before
    /// assuming a live server actually emits preview world-upgrade events.
    pub const V3_1_0: Self = Self::new(3, 1, 0);

    /// Creates a numeric `major.minor.patch` protocol version.
    ///
    /// No ordering or compatibility policy is inferred at construction time;
    /// use [`Self::is_at_least`] for a simple numeric comparison.
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Returns whether this version is numerically at least `minimum`.
    ///
    /// This is a lexicographic semantic-version comparison over major, minor,
    /// and patch. It does not imply that a specific method is available;
    /// discovery data remains the strongest source for per-method support.
    pub const fn is_at_least(self, minimum: Self) -> bool {
        self.major > minimum.major
            || (self.major == minimum.major && self.minor > minimum.minor)
            || (self.major == minimum.major
                && self.minor == minimum.minor
                && self.patch >= minimum.patch)
    }

    /// Parses `major.minor.patch`, ignoring an optional `-` or `+` suffix.
    ///
    /// For example, `3.1.0-snapshot.1` parses as `3.1.0`. Returns `None` when
    /// the numeric core is incomplete, contains extra components, or contains
    /// non-numeric fields.
    pub fn parse(value: &str) -> Option<Self> {
        value.parse().ok()
    }
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Parse error returned when text is not `major.minor.patch`.
///
/// Optional pre-release and build suffixes are accepted only after a complete
/// numeric core, such as `3.1.0-snapshot.1`.
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
        let numeric = trimmed.split(['-', '+']).next().unwrap_or(trimmed);
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

/// Optional behavior inferred from a discovery schema.
///
/// Features are convenience predicates derived from advertised method and
/// notification names and, where available, the advertised protocol version.
/// They are not an authoritative server promise; callers that need an exact
/// endpoint should also inspect [`Capabilities::methods`] or
/// [`Capabilities::notifications`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Feature {
    /// Authentication support or requirement is indicated by the protocol generation.
    Authentication,
    /// TLS is the expected default transport for the protocol generation.
    TlsByDefault,
    /// Notifications use the current `minecraft:notification/` namespace.
    MinecraftNotificationPrefix,
    /// The `minecraft:notification/server/activity` notification is available.
    ServerActivityNotification,
    /// Browser-oriented origin allowlisting is available.
    OriginAllowlist,
    /// Gamerules can use native JSON booleans and integers rather than strings.
    TypedGameruleValue,
    /// Discovery and status can work before full Minecraft server startup.
    PreStartDiscovery,
    /// World-upgrade lifecycle notifications are advertised.
    WorldUpgradeNotifications,
}

/// Parsed and lossless snapshot returned by `rpc.discover`.
///
/// The public collections are intentionally exposed so an application can
/// inspect unknown extension entries without waiting for a crate release.
/// `raw_schema` retains the exact JSON value returned by the server, while
/// `method_schemas` and `notification_schemas` preserve per-entry fragments
/// when the server provided them.
#[derive(Clone, Debug, PartialEq)]
pub struct Capabilities {
    /// Advertised MCSMP version when a parseable version was present.
    ///
    /// This is optional because discovery schemas may omit a version or use an
    /// unrecognized format. It describes MCSMP, not the Minecraft game version.
    pub protocol_version: Option<ProtocolVersion>,
    /// Complete JSON-RPC method names advertised by the server.
    ///
    /// Names are preserved exactly and can include extension namespaces.
    pub methods: BTreeSet<String>,
    /// Complete notification names advertised by the server.
    ///
    /// Names are normalized only where the supported parser can identify a
    /// historical official prefix; unknown extension names are retained.
    pub notifications: BTreeSet<String>,
    /// Convenience feature flags inferred from version and named capabilities.
    ///
    /// These are derived hints. Check `methods` or `notifications` for an
    /// exact endpoint-level decision.
    pub features: BTreeSet<Feature>,
    /// Unmodified JSON value returned by `rpc.discover`.
    ///
    /// Use this to inspect unknown fields or custom extension declarations
    /// without losing information during parsing.
    pub raw_schema: Value,
    /// Per-method schema fragments keyed by full method name, when available.
    pub method_schemas: BTreeMap<String, Value>,
    /// Per-notification schema fragments keyed by full name, when available.
    pub notification_schemas: BTreeMap<String, Value>,
}

impl Capabilities {
    /// Returns whether the exact JSON-RPC `method` name was advertised.
    ///
    /// Method matching is case-sensitive and uses the full name, for example
    /// `minecraft:server/status`.
    pub fn supports_method(&self, method: &str) -> bool {
        self.methods.contains(method)
    }
    /// Returns whether the exact notification method name was advertised.
    ///
    /// Notification matching is case-sensitive and uses the normalized full
    /// name, for example `minecraft:notification/players/joined`.
    pub fn supports_notification(&self, method: &str) -> bool {
        self.notifications.contains(method)
    }
    /// Returns whether this snapshot infers `feature`.
    ///
    /// This is a convenience predicate. For a precise custom extension,
    /// inspect [`Self::methods`] or [`Self::notifications`] directly.
    pub fn supports_feature(&self, feature: Feature) -> bool {
        self.features.contains(&feature)
    }
    /// Returns `Ok(())` when `feature` is inferred, otherwise an error.
    ///
    /// This helper is useful for guarding optional code paths. The returned
    /// [`crate::Error::UnsupportedFeature`] contains the requested feature.
    pub fn require_feature(&self, feature: Feature) -> Result<(), Error> {
        if self.supports_feature(feature) {
            Ok(())
        } else {
            Err(Error::UnsupportedFeature(feature))
        }
    }

    /// Builds a capability summary from a supported discovery-schema shape.
    ///
    /// This is public to support recorded schemas, tests, proxies, and custom
    /// discovery transports. The parser accepts method and notification entries
    /// represented either as name arrays or as maps/objects, retains all raw
    /// JSON, and tolerates unrecognized fields. Calling this function does not
    /// contact a server and does not validate that the schema came from MCSMP.
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
