use serde::{Deserialize, Serialize};

use crate::api::{call, params};
use crate::{Client, Message, Result, ServerState, SystemMessage};

/// Strongly typed access to `minecraft:server` operations.
#[derive(Clone, Debug)]
pub struct ServerApi {
    client: Client,
}

impl ServerApi {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Gets the server lifecycle state and online-player snapshot.
    pub async fn status(&self) -> Result<ServerState> {
        Ok(
            call::<StatusResult>(&self.client, "minecraft:server/status", None)
                .await?
                .status,
        )
    }

    /// Requests a world save, optionally asking the server to flush it to disk.
    ///
    /// Returns whether the server accepted or started the save operation.
    pub async fn save(&self, flush: bool) -> Result<bool> {
        Ok(call::<SavingResult>(
            &self.client,
            "minecraft:server/save",
            Some(params(SaveParams { flush })?),
        )
        .await?
        .saving)
    }

    /// Requests a normal save without an immediate flush.
    pub async fn save_default(&self) -> Result<bool> {
        self.save(false).await
    }

    /// Requests graceful server shutdown.
    ///
    /// Returns whether the server accepted or started the shutdown operation.
    pub async fn stop(&self) -> Result<bool> {
        Ok(
            call::<StoppingResult>(&self.client, "minecraft:server/stop", None)
                .await?
                .stopping,
        )
    }

    /// Sends a system message to all or selected players.
    ///
    /// Returns whether the server sent the message.
    pub async fn system_message(&self, message: SystemMessage) -> Result<bool> {
        Ok(call::<SentResult>(
            &self.client,
            "minecraft:server/system_message",
            Some(params(SystemMessageParams { message })?),
        )
        .await?
        .sent)
    }

    /// Sends literal system/chat text to all players.
    pub async fn chat(&self, text: impl Into<String>) -> Result<bool> {
        self.system_message(SystemMessage::chat(Message::literal(text)))
            .await
    }

    /// Sends literal action-bar text to all players.
    pub async fn action_bar(&self, text: impl Into<String>) -> Result<bool> {
        self.system_message(SystemMessage::action_bar(Message::literal(text)))
            .await
    }
}

#[derive(Deserialize)]
struct StatusResult {
    status: ServerState,
}

#[derive(Deserialize)]
struct SavingResult {
    saving: bool,
}

#[derive(Deserialize)]
struct StoppingResult {
    stopping: bool,
}

#[derive(Deserialize)]
struct SentResult {
    sent: bool,
}

#[derive(Serialize)]
struct SaveParams {
    flush: bool,
}

#[derive(Serialize)]
struct SystemMessageParams {
    message: SystemMessage,
}
