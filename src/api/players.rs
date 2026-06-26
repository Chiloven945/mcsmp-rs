//! Typed operations for the official `minecraft:players` namespace.

use serde::{Deserialize, Serialize};

use crate::api::{call, params};
use crate::{Client, KickPlayer, Message, PlayerRef, Result};

const ROOT: &str = "minecraft:players";

/// Typed handle for connected-player queries and disconnection requests.
///
/// Obtain this handle from [`crate::Client::players`]. The resource reports
/// only players currently connected at the time of the request; it is not an
/// account directory or allowlist.
#[derive(Clone, Debug)]
pub struct PlayersApi {
    client: Client,
}

impl PlayersApi {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Retrieves the current online-player snapshot.
    ///
    /// This maps to `minecraft:players`. The response is a point-in-time
    /// snapshot; subscribe to player notifications with [`crate::Client::subscribe`]
    /// when an application also needs to observe joins and leaves between
    /// polls.
    pub async fn list(&self) -> Result<Vec<PlayerRef>> {
        Ok(call::<PlayersResult>(&self.client, ROOT, None)
            .await?
            .players)
    }

    /// Disconnects the supplied players.
    ///
    /// This maps to `minecraft:players/kick`. Each [`KickPlayer`] contains a
    /// selector and may contain a literal or translatable disconnect message.
    /// The returned selectors identify players that the server actually
    /// disconnected; they need not match the input count if a target was
    /// already offline or could not be resolved.
    ///
    /// Kicking is non-idempotent. If a request fails after reaching the
    /// network, inspect server state rather than blindly retrying it.
    pub async fn kick(
        &self,
        requests: impl IntoIterator<Item = KickPlayer>,
    ) -> Result<Vec<PlayerRef>> {
        let kick: Vec<_> = requests.into_iter().collect();
        Ok(call::<KickedResult>(
            &self.client,
            "minecraft:players/kick",
            Some(params(KickParams { kick })?),
        )
        .await?
        .kicked)
    }

    /// Disconnects one player without a custom message.
    ///
    /// This is a convenience wrapper around [`Self::kick`] using
    /// [`KickPlayer::new`].
    pub async fn kick_player(&self, player: PlayerRef) -> Result<Vec<PlayerRef>> {
        self.kick([KickPlayer::new(player)]).await
    }

    /// Disconnects one player with a custom literal or translatable message.
    ///
    /// This is a convenience wrapper around [`Self::kick`] using
    /// [`KickPlayer::with_message`].
    pub async fn kick_with_message(
        &self,
        player: PlayerRef,
        message: Message,
    ) -> Result<Vec<PlayerRef>> {
        self.kick([KickPlayer::with_message(player, message)]).await
    }
}

#[derive(Deserialize)]
struct PlayersResult {
    players: Vec<PlayerRef>,
}

#[derive(Deserialize)]
struct KickedResult {
    kicked: Vec<PlayerRef>,
}

#[derive(Serialize)]
struct KickParams {
    kick: Vec<KickPlayer>,
}
