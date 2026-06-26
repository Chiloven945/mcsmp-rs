//! Typed operations for the official `minecraft:bans` namespace.
//!
//! User bans target Minecraft player accounts through [`crate::PlayerRef`].
//! They are distinct from IP bans, which are exposed through
//! [`crate::IpBansApi`].

use serde::{Deserialize, Serialize};

use crate::api::{call, params};
use crate::{Client, PlayerRef, Result, UserBan};

const ROOT: &str = "minecraft:bans";

/// Typed handle for the official user-ban resource.
///
/// Obtain this handle from [`crate::Client::bans`]. Mutating calls return the
/// entire user-ban list after the operation, allowing callers to treat the
/// response as an authoritative replacement for any local cache.
#[derive(Clone, Debug)]
pub struct BansApi {
    client: Client,
}

impl BansApi {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Retrieves the current user-ban list with `minecraft:bans`.
    ///
    /// Each entry contains a player selector and may include a reason, source,
    /// and ISO-8601 expiry instant. `expires: None` denotes a permanent ban.
    pub async fn list(&self) -> Result<Vec<UserBan>> {
        Ok(call::<BanListResult>(&self.client, ROOT, None)
            .await?
            .banlist)
    }

    /// Replaces the entire user-ban list with `bans`.
    ///
    /// This destructive operation maps to `minecraft:bans/set`; an empty
    /// iterator clears every user ban. Use [`Self::add`] or [`Self::remove`]
    /// when changing a subset of the list is sufficient.
    pub async fn set(&self, bans: impl IntoIterator<Item = UserBan>) -> Result<Vec<UserBan>> {
        let bans: Vec<_> = bans.into_iter().collect();
        Ok(call::<BanListResult>(
            &self.client,
            "minecraft:bans/set",
            Some(params(BansParams { bans })?),
        )
        .await?
        .banlist)
    }

    /// Adds one or more user-ban entries.
    ///
    /// This maps to `minecraft:bans/add`. The server decides how matching
    /// existing entries are updated or deduplicated. A ban may disconnect an
    /// affected online player, so treat this operation as non-idempotent when
    /// deciding whether to issue it again after a connection failure.
    pub async fn add(&self, bans: impl IntoIterator<Item = UserBan>) -> Result<Vec<UserBan>> {
        let add: Vec<_> = bans.into_iter().collect();
        Ok(call::<BanListResult>(
            &self.client,
            "minecraft:bans/add",
            Some(params(AddParams { add })?),
        )
        .await?
        .banlist)
    }

    /// Removes bans associated with `players`.
    ///
    /// This maps to `minecraft:bans/remove`. Selectors are resolved by the
    /// server, so callers may use UUIDs, names, or both. The returned list is
    /// the complete user-ban snapshot after removal.
    pub async fn remove(
        &self,
        players: impl IntoIterator<Item = PlayerRef>,
    ) -> Result<Vec<UserBan>> {
        let remove: Vec<_> = players.into_iter().collect();
        Ok(call::<BanListResult>(
            &self.client,
            "minecraft:bans/remove",
            Some(params(RemoveParams { remove })?),
        )
        .await?
        .banlist)
    }

    /// Removes all user bans with `minecraft:bans/clear`.
    ///
    /// Returns the complete resulting ban list, normally empty.
    pub async fn clear(&self) -> Result<Vec<UserBan>> {
        Ok(
            call::<BanListResult>(&self.client, "minecraft:bans/clear", None)
                .await?
                .banlist,
        )
    }
}

#[derive(Deserialize)]
struct BanListResult {
    banlist: Vec<UserBan>,
}

#[derive(Serialize)]
struct BansParams {
    bans: Vec<UserBan>,
}

#[derive(Serialize)]
struct AddParams {
    add: Vec<UserBan>,
}

#[derive(Serialize)]
struct RemoveParams {
    remove: Vec<PlayerRef>,
}
