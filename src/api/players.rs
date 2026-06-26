use serde::{Deserialize, Serialize};

use crate::api::{call, params};
use crate::{Client, KickPlayer, Message, PlayerRef, Result};

const ROOT: &str = "minecraft:players";

/// Strongly typed access to `minecraft:players` operations.
#[derive(Clone, Debug)]
pub struct PlayersApi {
    client: Client,
}

impl PlayersApi {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Gets the current online-player snapshot.
    pub async fn list(&self) -> Result<Vec<PlayerRef>> {
        Ok(call::<PlayersResult>(&self.client, ROOT, None)
            .await?
            .players)
    }

    /// Kicks the supplied players and returns the selectors that the server
    /// actually disconnected.
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

    /// Kicks one player without a custom disconnect message.
    pub async fn kick_player(&self, player: PlayerRef) -> Result<Vec<PlayerRef>> {
        self.kick([KickPlayer::new(player)]).await
    }

    /// Kicks one player and sends the supplied custom disconnect message.
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
