use std::net::IpAddr;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use super::player::{ensure_not_blank, ModelError};
use super::PlayerRef;

/// A user-ban entry returned by, or sent to, `minecraft:bans` endpoints.
///
/// `expires` is an ISO-8601 instant in the representation accepted by the
/// server. `None` denotes a permanent ban.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserBan {
    /// The player whose account is banned.
    pub player: PlayerRef,
    /// An optional human-readable reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// An optional actor or source that created the ban.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// An optional ISO-8601 expiration instant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires: Option<String>,
}

impl UserBan {
    /// Creates a permanent user ban with no extra metadata.
    pub fn permanent(player: PlayerRef) -> Self {
        Self {
            player,
            reason: None,
            source: None,
            expires: None,
        }
    }

    /// Creates a permanent user ban with a non-blank reason.
    pub fn with_reason(player: PlayerRef, reason: impl Into<String>) -> Result<Self, ModelError> {
        let reason = reason.into();
        ensure_not_blank("reason", &reason)?;
        Ok(Self {
            player,
            reason: Some(reason),
            source: None,
            expires: None,
        })
    }

    /// Adds a non-blank source string to this ban.
    pub fn with_source(mut self, source: impl Into<String>) -> Result<Self, ModelError> {
        let source = source.into();
        ensure_not_blank("source", &source)?;
        self.source = Some(source);
        Ok(self)
    }

    /// Adds a non-blank ISO-8601 expiration instant to this ban.
    pub fn expiring_at(mut self, expires: impl Into<String>) -> Result<Self, ModelError> {
        let expires = expires.into();
        ensure_not_blank("expires", &expires)?;
        self.expires = Some(expires);
        Ok(self)
    }
}

/// A resolved IP-ban entry returned by, or sent to, `minecraft:ip_bans`.
///
/// `expires` is an ISO-8601 instant; `None` denotes a permanent ban.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpBan {
    /// The concrete network address that is banned.
    pub ip: IpAddr,
    /// An optional human-readable reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// An optional actor or source that created the ban.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// An optional ISO-8601 expiration instant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires: Option<String>,
}

impl IpBan {
    /// Creates a permanent IP ban with no extra metadata.
    pub fn permanent(ip: IpAddr) -> Self {
        Self {
            ip,
            reason: None,
            source: None,
            expires: None,
        }
    }

    /// Creates a permanent IP ban with a non-blank reason.
    pub fn with_reason(ip: IpAddr, reason: impl Into<String>) -> Result<Self, ModelError> {
        let reason = reason.into();
        ensure_not_blank("reason", &reason)?;
        Ok(Self {
            ip,
            reason: Some(reason),
            source: None,
            expires: None,
        })
    }

    /// Adds a non-blank source string to this ban.
    pub fn with_source(mut self, source: impl Into<String>) -> Result<Self, ModelError> {
        let source = source.into();
        ensure_not_blank("source", &source)?;
        self.source = Some(source);
        Ok(self)
    }

    /// Adds a non-blank ISO-8601 expiration instant to this ban.
    pub fn expiring_at(mut self, expires: impl Into<String>) -> Result<Self, ModelError> {
        let expires = expires.into();
        ensure_not_blank("expires", &expires)?;
        self.expires = Some(expires);
        Ok(self)
    }
}

/// A request to create an IP ban by direct address, player selector, or both.
///
/// The server resolves player selectors into a concrete address when possible.
/// It may use the direct IP address when both fields are set.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingIpBan {
    #[serde(skip_serializing_if = "Option::is_none")]
    ip: Option<IpAddr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    player: Option<PlayerRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires: Option<String>,
}

impl IncomingIpBan {
    /// Creates a direct-address IP-ban request.
    pub const fn ip(ip: IpAddr) -> Self {
        Self {
            ip: Some(ip),
            player: None,
            reason: None,
            source: None,
            expires: None,
        }
    }

    /// Creates an IP-ban request that asks the server to resolve a player's
    /// current address.
    pub fn player(player: PlayerRef) -> Self {
        Self {
            ip: None,
            player: Some(player),
            reason: None,
            source: None,
            expires: None,
        }
    }

    /// Creates an IP-ban request containing an optional direct address and
    /// optional player selector.
    pub fn new(ip: Option<IpAddr>, player: Option<PlayerRef>) -> Result<Self, ModelError> {
        if ip.is_none() && player.is_none() {
            return Err(ModelError::MissingIpBanTarget);
        }
        Ok(Self {
            ip,
            player,
            reason: None,
            source: None,
            expires: None,
        })
    }

    /// Returns the direct IP address selected by this request, when present.
    pub const fn address(&self) -> Option<IpAddr> {
        self.ip
    }

    /// Returns the player selector selected by this request, when present.
    pub fn player_selector(&self) -> Option<&PlayerRef> {
        self.player.as_ref()
    }

    /// Returns the optional human-readable ban reason.
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    /// Returns the optional actor or source that created this ban request.
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    /// Returns the optional ISO-8601 expiration instant.
    pub fn expires(&self) -> Option<&str> {
        self.expires.as_deref()
    }

    /// Adds a non-blank human-readable reason.
    pub fn with_reason(mut self, reason: impl Into<String>) -> Result<Self, ModelError> {
        let reason = reason.into();
        ensure_not_blank("reason", &reason)?;
        self.reason = Some(reason);
        Ok(self)
    }

    /// Adds a non-blank actor or source string.
    pub fn with_source(mut self, source: impl Into<String>) -> Result<Self, ModelError> {
        let source = source.into();
        ensure_not_blank("source", &source)?;
        self.source = Some(source);
        Ok(self)
    }

    /// Adds a non-blank ISO-8601 expiration instant.
    pub fn expiring_at(mut self, expires: impl Into<String>) -> Result<Self, ModelError> {
        let expires = expires.into();
        ensure_not_blank("expires", &expires)?;
        self.expires = Some(expires);
        Ok(self)
    }
}

impl<'de> Deserialize<'de> for IncomingIpBan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            ip: Option<IpAddr>,
            player: Option<PlayerRef>,
            reason: Option<String>,
            source: Option<String>,
            expires: Option<String>,
        }

        let wire = Wire::deserialize(deserializer)?;
        let mut value = Self::new(wire.ip, wire.player).map_err(D::Error::custom)?;
        if let Some(reason) = wire.reason {
            value = value.with_reason(reason).map_err(D::Error::custom)?;
        }
        if let Some(source) = wire.source {
            value = value.with_source(source).map_err(D::Error::custom)?;
        }
        if let Some(expires) = wire.expires {
            value = value.expiring_at(expires).map_err(D::Error::custom)?;
        }
        Ok(value)
    }
}
