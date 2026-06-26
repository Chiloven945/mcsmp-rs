use serde::{Deserialize, Serialize};

use crate::api::{call, params};
use crate::{Client, PlayerRef, Result, UserBan};

const ROOT: &str = "minecraft:bans";

/// Strongly typed access to `minecraft:bans` operations.
#[derive(Clone, Debug)]
pub struct BansApi {
    client: Client,
}

impl BansApi {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Gets the current user-ban list.
    pub async fn list(&self) -> Result<Vec<UserBan>> {
        Ok(call::<BanListResult>(&self.client, ROOT, None)
            .await?
            .banlist)
    }

    /// Replaces the user-ban list and returns the resulting server snapshot.
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

    /// Adds user-ban entries and returns the resulting server snapshot.
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

    /// Removes user bans for the supplied players and returns the resulting
    /// server snapshot.
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

    /// Clears all user bans and returns the resulting server snapshot.
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
