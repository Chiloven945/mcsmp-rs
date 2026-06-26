//! Typed operations for the official `minecraft:allowlist` namespace.
//!
//! The methods in this module return the server's resulting allowlist snapshot.
//! Treat that snapshot as authoritative: Minecraft may normalize player
//! selectors, ignore duplicates, or apply its own persistence rules.

use serde::{Deserialize, Serialize};

use crate::api::{call, params};
use crate::{Client, PlayerRef, Result};

const ROOT: &str = "minecraft:allowlist";

/// Typed handle for the official allowlist resource.
///
/// Obtain a handle from [`crate::Client::allowlist`]. The handle is cheap to
/// clone and shares the client's WebSocket session; it does not create another
/// connection.
///
/// All mutating operations return the full allowlist *after* Minecraft has
/// applied the request. This makes it possible to update local UI or cache
/// state without issuing a separate `list` request.
#[derive(Clone, Debug)]
pub struct AllowlistApi {
    client: Client,
}

impl AllowlistApi {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Retrieves the current allowlist with `minecraft:allowlist`.
    ///
    /// The returned selectors may contain UUIDs, names, or both, according to
    /// the server's stored player information. The method does not report
    /// whether allowlist enforcement is enabled; query
    /// [`crate::ServerSettingsApi::use_allowlist`] for that setting.
    pub async fn list(&self) -> Result<Vec<PlayerRef>> {
        Ok(call::<AllowlistResult>(&self.client, ROOT, None)
            .await?
            .allowlist)
    }

    /// Replaces the entire allowlist with `players`.
    ///
    /// This maps to `minecraft:allowlist/set`. Passing an empty iterator is
    /// valid and removes every entry. The operation is destructive: callers
    /// that only want to grant access to extra users should normally prefer
    /// [`Self::add`].
    ///
    /// Returns the authoritative allowlist snapshot after the server applies
    /// the replacement.
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

    /// Adds `players` to the allowlist.
    ///
    /// This maps to `minecraft:allowlist/add`. Duplicate or already-present
    /// selectors are interpreted by the server, so callers should use the
    /// returned snapshot rather than assuming one entry was created per input.
    ///
    /// When allowlist enforcement is enabled, removing a currently connected
    /// player can cause that player to be kicked. Adding a player does not
    /// itself toggle enforcement.
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

    /// Removes `players` from the allowlist.
    ///
    /// This maps to `minecraft:allowlist/remove`. A selector can identify a
    /// player by UUID, name, or both. If the server has allowlist enforcement
    /// enabled, it may immediately disconnect an online player removed by this
    /// operation.
    ///
    /// Returns the resulting full allowlist.
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

    /// Removes every entry from the allowlist.
    ///
    /// This maps to `minecraft:allowlist/clear` and returns the resulting
    /// snapshot, which is normally empty. This method does not disable
    /// allowlist use; disabling that policy is a separate
    /// `ServerSettingsApi::set_use_allowlist` operation.
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
