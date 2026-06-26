use std::net::IpAddr;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use super::PlayerRef;
use super::player::{ModelError, ensure_not_blank};

/// User-ban entry used by the `minecraft:bans` resource.
///
/// A user ban targets a [`crate::PlayerRef`] rather than a network address.
/// `reason` and `source` are optional metadata. `expires` is an ISO-8601
/// instant accepted by Minecraft; `None` means that the ban is permanent.
/// The crate validates only that provided text is non-blank, leaving timestamp
/// syntax and account resolution to the server.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserBan {
    /// Player selector identifying the banned account.
    pub player: PlayerRef,
    /// Optional non-blank human-readable reason stored with the ban.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Optional non-blank actor, tool, or source label that created the ban.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Optional ISO-8601 expiration instant; `None` denotes a permanent ban.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires: Option<String>,
}

impl UserBan {
    /// Creates a permanent user ban with no reason or source metadata.
    pub fn permanent(player: PlayerRef) -> Self {
        Self {
            player,
            reason: None,
            source: None,
            expires: None,
        }
    }

    /// Creates a permanent user ban with a non-blank reason.
    ///
    /// Returns [`ModelError::BlankField`] when `reason` is empty or
    /// whitespace-only.
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

    /// Adds a non-blank source label and returns the updated ban.
    ///
    /// This builder-style method returns [`ModelError::BlankField`] for empty
    /// or whitespace-only input.
    pub fn with_source(mut self, source: impl Into<String>) -> Result<Self, ModelError> {
        let source = source.into();
        ensure_not_blank("source", &source)?;
        self.source = Some(source);
        Ok(self)
    }

    /// Adds a non-blank expiration string and returns the updated ban.
    ///
    /// The value should be an ISO-8601 instant accepted by Minecraft. Syntax
    /// beyond the non-blank check is validated by the server.
    pub fn expiring_at(mut self, expires: impl Into<String>) -> Result<Self, ModelError> {
        let expires = expires.into();
        ensure_not_blank("expires", &expires)?;
        self.expires = Some(expires);
        Ok(self)
    }
}

/// Resolved network-address ban entry used by `minecraft:ip_bans`.
///
/// Unlike [`IncomingIpBan`], this type always contains a concrete
/// [`std::net::IpAddr`]. `expires` is an ISO-8601 instant accepted by
/// Minecraft; `None` denotes a permanent ban.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpBan {
    /// Concrete IPv4 or IPv6 address that is banned.
    pub ip: IpAddr,
    /// Optional non-blank human-readable reason stored with the ban.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Optional non-blank actor, tool, or source label that created the ban.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Optional ISO-8601 expiration instant; `None` denotes a permanent ban.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires: Option<String>,
}

impl IpBan {
    /// Creates a permanent IP ban with no reason or source metadata.
    pub fn permanent(ip: IpAddr) -> Self {
        Self {
            ip,
            reason: None,
            source: None,
            expires: None,
        }
    }

    /// Creates a permanent IP ban with a non-blank reason.
    ///
    /// Returns [`ModelError::BlankField`] for empty or whitespace-only text.
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

    /// Adds a non-blank source label and returns the updated ban.
    ///
    /// This builder-style method returns [`ModelError::BlankField`] for empty
    /// or whitespace-only input.
    pub fn with_source(mut self, source: impl Into<String>) -> Result<Self, ModelError> {
        let source = source.into();
        ensure_not_blank("source", &source)?;
        self.source = Some(source);
        Ok(self)
    }

    /// Adds a non-blank expiration string and returns the updated ban.
    ///
    /// The value should be an ISO-8601 instant accepted by Minecraft. Syntax
    /// beyond the non-blank check is validated by the server.
    pub fn expiring_at(mut self, expires: impl Into<String>) -> Result<Self, ModelError> {
        let expires = expires.into();
        ensure_not_blank("expires", &expires)?;
        self.expires = Some(expires);
        Ok(self)
    }
}

/// Input for creating an IP ban by direct address, player selector, or both.
///
/// The server can resolve a [`crate::PlayerRef`] to the player's current
/// address. When both fields are supplied, the direct address is preserved in
/// the request and the server determines final resolution. The result of
/// `IpBansApi::add` is always a resolved [`IpBan`] with a concrete address.
///
/// At least one target is required. Optional reason, source, and expiry fields
/// use the same semantics as [`IpBan`].
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
    /// Creates a request targeting one concrete IPv4 or IPv6 address.
    pub const fn ip(ip: IpAddr) -> Self {
        Self {
            ip: Some(ip),
            player: None,
            reason: None,
            source: None,
            expires: None,
        }
    }

    /// Creates a request asking the server to resolve a player's address.
    ///
    /// Resolution happens remotely and may fail or produce no ban if the
    /// server cannot obtain an address for the selected player.
    pub fn player(player: PlayerRef) -> Self {
        Self {
            ip: None,
            player: Some(player),
            reason: None,
            source: None,
            expires: None,
        }
    }

    /// Creates an IP-ban request with an optional address and player selector.
    ///
    /// At least one argument must be present or
    /// [`ModelError::MissingIpBanTarget`] is returned.
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

    /// Returns the explicitly supplied IP address, when present.
    pub const fn address(&self) -> Option<IpAddr> {
        self.ip
    }

    /// Returns the player selector used for server-side address resolution.
    pub fn player_selector(&self) -> Option<&PlayerRef> {
        self.player.as_ref()
    }

    /// Returns the optional human-readable ban reason.
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    /// Returns the optional actor, tool, or source label.
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    /// Returns the optional ISO-8601 expiration string.
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

    /// Adds a non-blank actor, tool, or source label.
    pub fn with_source(mut self, source: impl Into<String>) -> Result<Self, ModelError> {
        let source = source.into();
        ensure_not_blank("source", &source)?;
        self.source = Some(source);
        Ok(self)
    }

    /// Adds a non-blank expiration string.
    ///
    /// The server validates ISO-8601 syntax when the request is sent.
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
