//! Typed operations for the official `minecraft:server` namespace.

use serde::{Deserialize, Serialize};

use crate::api::{call, params};
use crate::{Client, Message, Result, ServerState, SystemMessage};

/// Typed handle for server status, saving, shutdown, and system messages.
///
/// Obtain this handle from [`crate::Client::server`]. Lifecycle methods can
/// change server availability; applications should consume their boolean
/// acknowledgement as "accepted or started", not as proof that a long-running
/// operation has already completed.
#[derive(Clone, Debug)]
pub struct ServerApi {
    client: Client,
}

impl ServerApi {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Retrieves a point-in-time lifecycle and online-player snapshot.
    ///
    /// This maps to `minecraft:server/status`. The returned
    /// [`ServerState::version`] is the Minecraft *game* version, not the MCSMP
    /// protocol version. Use [`crate::Client::discover`] and
    /// [`crate::Capabilities::protocol_version`] for the latter.
    pub async fn status(&self) -> Result<ServerState> {
        Ok(
            call::<StatusResult>(&self.client, "minecraft:server/status", None)
                .await?
                .status,
        )
    }

    /// Requests a world save.
    ///
    /// This maps to `minecraft:server/save`. Set `flush` to `true` to request
    /// that the server flush persisted world data to disk; set it to `false`
    /// for the normal save behavior. The returned boolean means the server
    /// accepted or began saving. Subscribe to `Event::ServerSaving` and
    /// `Event::ServerSaved` when completion matters.
    ///
    /// Save requests should not be automatically retried after a disconnect,
    /// because the first request may already have reached the server.
    pub async fn save(&self, flush: bool) -> Result<bool> {
        Ok(call::<SavingResult>(
            &self.client,
            "minecraft:server/save",
            Some(params(SaveParams { flush })?),
        )
        .await?
        .saving)
    }

    /// Requests a normal world save without forcing an immediate disk flush.
    ///
    /// This is equivalent to calling [`Self::save`] with `false`.
    pub async fn save_default(&self) -> Result<bool> {
        self.save(false).await
    }

    /// Requests graceful dedicated-server shutdown.
    ///
    /// This maps to `minecraft:server/stop`. The returned boolean means the
    /// server accepted or began shutdown; the current WebSocket may close as a
    /// consequence. Treat this as a non-idempotent administrative action.
    pub async fn stop(&self) -> Result<bool> {
        Ok(
            call::<StoppingResult>(&self.client, "minecraft:server/stop", None)
                .await?
                .stopping,
        )
    }

    /// Sends a system message to all players or selected recipients.
    ///
    /// This maps to `minecraft:server/system_message`. Use
    /// [`SystemMessage::chat`] for normal chat-style messages,
    /// [`SystemMessage::action_bar`] for overlays, and [`SystemMessage::to`]
    /// to restrict recipients. The returned boolean means the server accepted
    /// the message for delivery.
    pub async fn system_message(&self, message: SystemMessage) -> Result<bool> {
        Ok(call::<SentResult>(
            &self.client,
            "minecraft:server/system_message",
            Some(params(SystemMessageParams { message })?),
        )
        .await?
        .sent)
    }

    /// Sends literal chat-style system text to all applicable players.
    ///
    /// This is a convenience wrapper around [`Self::system_message`] with
    /// `SystemMessage::chat(Message::literal(text))`.
    pub async fn chat(&self, text: impl Into<String>) -> Result<bool> {
        self.system_message(SystemMessage::chat(Message::literal(text)))
            .await
    }

    /// Sends literal action-bar text to all applicable players.
    ///
    /// This is a convenience wrapper around [`Self::system_message`] with
    /// `SystemMessage::action_bar(Message::literal(text))`.
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
