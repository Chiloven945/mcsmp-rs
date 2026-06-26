use serde::{Deserialize, Serialize};

use crate::api::{call, params};
use crate::{Client, PlayerRef, Result};

const ROOT: &str = "minecraft:allowlist";

/// Strongly typed access to `minecraft:allowlist` operations.
#[derive(Clone, Debug)]
pub struct AllowlistApi {
    client: Client,
}

impl AllowlistApi {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Gets the current allowlist.
    pub async fn list(&self) -> Result<Vec<PlayerRef>> {
        Ok(call::<AllowlistResult>(&self.client, ROOT, None)
            .await?
            .allowlist)
    }

    /// Replaces the allowlist with the supplied players and returns the new
    /// server snapshot.
    pub async fn set(
        &self,
        players: impl IntoIterator<Item = PlayerRef>,
    ) -> Result<Vec<PlayerRef>> {
        let players: Vec<_> = players.into_iter().collect();
        Ok(call::<AllowlistResult>(
            &self.client,
            "minecraft:allowlist/set",
            Some(params(PlayersParams { players })?),
        )
        .await?
        .allowlist)
    }

    /// Adds players to the allowlist and returns the new server snapshot.
    pub async fn add(
        &self,
        players: impl IntoIterator<Item = PlayerRef>,
    ) -> Result<Vec<PlayerRef>> {
        let add: Vec<_> = players.into_iter().collect();
        Ok(call::<AllowlistResult>(
            &self.client,
            "minecraft:allowlist/add",
            Some(params(AddParams { add })?),
        )
        .await?
        .allowlist)
    }

    /// Removes players from the allowlist and returns the new server snapshot.
    pub async fn remove(
        &self,
        players: impl IntoIterator<Item = PlayerRef>,
    ) -> Result<Vec<PlayerRef>> {
        let remove: Vec<_> = players.into_iter().collect();
        Ok(call::<AllowlistResult>(
            &self.client,
            "minecraft:allowlist/remove",
            Some(params(RemoveParams { remove })?),
        )
        .await?
        .allowlist)
    }

    /// Clears the allowlist and returns the resulting server snapshot.
    pub async fn clear(&self) -> Result<Vec<PlayerRef>> {
        Ok(
            call::<AllowlistResult>(&self.client, "minecraft:allowlist/clear", None)
                .await?
                .allowlist,
        )
    }
}

#[derive(Deserialize)]
struct AllowlistResult {
    allowlist: Vec<PlayerRef>,
}

#[derive(Serialize)]
struct PlayersParams {
    players: Vec<PlayerRef>,
}

#[derive(Serialize)]
struct AddParams {
    add: Vec<PlayerRef>,
}

#[derive(Serialize)]
struct RemoveParams {
    remove: Vec<PlayerRef>,
}
