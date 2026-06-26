use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::{Client, Difficulty, Error, GameMode, Result};

const ROOT: &str = "minecraft:serversettings";

/// Typed handle for live dedicated-server settings in `minecraft:serversettings`.
///
/// Obtain this handle from [`crate::Client::server_settings`]. Getter methods
/// query the setting currently active in the running server; they do not read
/// `server.properties` directly. Setter methods change the active setting and
/// return the value acknowledged by Minecraft.
///
/// Numeric ranges are deliberately validated by the server rather than guessed
/// client-side. This preserves compatibility with server versions that change
/// their permitted bounds. A successful setter response is an acknowledgement,
/// not a persisted configuration-file edit: consult the Minecraft server's
/// own configuration semantics when deciding whether a restart is required.
#[derive(Clone, Debug)]
pub struct ServerSettingsApi {
    client: Client,
}

impl ServerSettingsApi {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Returns whether automatic world saving is currently enabled.
    ///
    /// Sends `minecraft:serversettings/autosave` and decodes its `enabled`
    /// result field. Disabling automatic saving does not itself issue a save;
    /// call [`crate::ServerApi::save`] before maintenance when a final save is
    /// required.
    pub async fn autosave(&self) -> Result<bool> {
        self.get("autosave", "enabled").await
    }

    /// Enables or disables automatic world saving.
    ///
    /// Sends `minecraft:serversettings/autosave/set` with `{ "enable": ... }`
    /// and returns the server's `enabled` acknowledgement.
    pub async fn set_autosave(&self, enabled: bool) -> Result<bool> {
        self.set("autosave", "enable", enabled, "enabled").await
    }

    /// Returns the active world difficulty.
    ///
    /// Sends `minecraft:serversettings/difficulty` and returns one of the
    /// MCSMP [`crate::Difficulty`] values.
    pub async fn difficulty(&self) -> Result<Difficulty> {
        self.get("difficulty", "difficulty").await
    }

    /// Sets the active world difficulty.
    ///
    /// Sends `minecraft:serversettings/difficulty/set` with the lowercase
    /// wire representation of `difficulty` and returns the acknowledged
    /// [`crate::Difficulty`].
    pub async fn set_difficulty(&self, difficulty: Difficulty) -> Result<Difficulty> {
        self.set("difficulty", "difficulty", difficulty, "difficulty")
            .await
    }

    /// Returns whether allowlist removals are immediately enforced.
    ///
    /// Sends `minecraft:serversettings/enforce_allowlist`. When true, removing
    /// a currently connected player through `AllowlistApi::remove` can kick
    /// that player immediately.
    pub async fn enforce_allowlist(&self) -> Result<bool> {
        self.get("enforce_allowlist", "enforced").await
    }

    /// Enables or disables immediate enforcement after allowlist removal.
    ///
    /// Sends `minecraft:serversettings/enforce_allowlist/set` with
    /// `{ "enforce": ... }` and returns `enforced`.
    pub async fn set_enforce_allowlist(&self, enforced: bool) -> Result<bool> {
        self.set("enforce_allowlist", "enforce", enforced, "enforced")
            .await
    }

    /// Returns whether the allowlist restricts new player joins.
    ///
    /// Sends `minecraft:serversettings/use_allowlist`. This policy is distinct
    /// from immediate removal enforcement exposed by [`Self::enforce_allowlist`].
    pub async fn use_allowlist(&self) -> Result<bool> {
        self.get("use_allowlist", "used").await
    }

    /// Enables or disables allowlist-based join restriction.
    ///
    /// Sends `minecraft:serversettings/use_allowlist/set` with `{ "use": ... }`
    /// and returns the `used` acknowledgement.
    pub async fn set_use_allowlist(&self, used: bool) -> Result<bool> {
        self.set("use_allowlist", "use", used, "used").await
    }

    /// Returns the current maximum number of concurrent players.
    ///
    /// Sends `minecraft:serversettings/max_players` and decodes the `max`
    /// result field. Minecraft performs final validation of this number.
    pub async fn max_players(&self) -> Result<i32> {
        self.get("max_players", "max").await
    }

    /// Sets the maximum number of concurrent players.
    ///
    /// Sends `minecraft:serversettings/max_players/set` with `{ "max": ... }`
    /// and returns the value accepted by the server.
    pub async fn set_max_players(&self, max_players: i32) -> Result<i32> {
        self.set("max_players", "max", max_players, "max").await
    }

    /// Returns the idle-empty-server pause delay in seconds.
    ///
    /// Sends `minecraft:serversettings/pause_when_empty_seconds`. The exact
    /// behavior of pausing is determined by the server version and its current
    /// world state.
    pub async fn pause_when_empty_seconds(&self) -> Result<i32> {
        self.get("pause_when_empty_seconds", "seconds").await
    }

    /// Sets the idle-empty-server pause delay in seconds.
    ///
    /// Sends `minecraft:serversettings/pause_when_empty_seconds/set` with
    /// `{ "seconds": ... }`. Range and sentinel-value validation is performed
    /// by Minecraft.
    pub async fn set_pause_when_empty_seconds(&self, seconds: i32) -> Result<i32> {
        self.set("pause_when_empty_seconds", "seconds", seconds, "seconds")
            .await
    }

    /// Returns the idle-player automatic kick timeout in seconds.
    ///
    /// Sends `minecraft:serversettings/player_idle_timeout`. This is a server
    /// policy setting and does not inspect individual player activity.
    pub async fn player_idle_timeout(&self) -> Result<i32> {
        self.get("player_idle_timeout", "seconds").await
    }

    /// Sets the idle-player automatic kick timeout in seconds.
    ///
    /// Sends `minecraft:serversettings/player_idle_timeout/set` with
    /// `{ "seconds": ... }` and returns the accepted value.
    pub async fn set_player_idle_timeout(&self, seconds: i32) -> Result<i32> {
        self.set("player_idle_timeout", "seconds", seconds, "seconds")
            .await
    }

    /// Returns whether flight is permitted for Survival-mode players.
    ///
    /// Sends `minecraft:serversettings/allow_flight` and returns its `allowed`
    /// field. This setting does not change player game modes.
    pub async fn allow_flight(&self) -> Result<bool> {
        self.get("allow_flight", "allowed").await
    }

    /// Enables or disables Survival-mode flight permission.
    ///
    /// Sends `minecraft:serversettings/allow_flight/set` with
    /// `{ "allowed": ... }` and returns the server acknowledgement.
    pub async fn set_allow_flight(&self, allowed: bool) -> Result<bool> {
        self.set("allow_flight", "allowed", allowed, "allowed")
            .await
    }

    /// Returns the current message of the day shown in status responses.
    ///
    /// Sends `minecraft:serversettings/motd` and returns the `message` field.
    pub async fn motd(&self) -> Result<String> {
        self.get("motd", "message").await
    }

    /// Sets the message of the day shown in status responses.
    ///
    /// Sends `minecraft:serversettings/motd/set` with `{ "message": ... }`.
    /// The server owns validation and any formatting interpretation.
    pub async fn set_motd(&self, message: impl Into<String>) -> Result<String> {
        self.set("motd", "message", message.into(), "message").await
    }

    /// Returns the spawn-protection radius in blocks.
    ///
    /// Sends `minecraft:serversettings/spawn_protection_radius`. This setting
    /// governs the protected area where only operators may edit.
    pub async fn spawn_protection_radius(&self) -> Result<i32> {
        self.get("spawn_protection_radius", "radius").await
    }

    /// Sets the spawn-protection radius in blocks.
    ///
    /// Sends `minecraft:serversettings/spawn_protection_radius/set` with
    /// `{ "radius": ... }` and returns the accepted radius.
    pub async fn set_spawn_protection_radius(&self, radius: i32) -> Result<i32> {
        self.set("spawn_protection_radius", "radius", radius, "radius")
            .await
    }

    /// Returns whether joiners are forced into the configured default game mode.
    ///
    /// Sends `minecraft:serversettings/force_game_mode` and decodes `forced`.
    pub async fn force_game_mode(&self) -> Result<bool> {
        self.get("force_game_mode", "forced").await
    }

    /// Enables or disables forcing the default game mode for players.
    ///
    /// Sends `minecraft:serversettings/force_game_mode/set` with
    /// `{ "force": ... }` and returns `forced`.
    pub async fn set_force_game_mode(&self, forced: bool) -> Result<bool> {
        self.set("force_game_mode", "force", forced, "forced").await
    }

    /// Returns the dedicated server's default game mode.
    ///
    /// Sends `minecraft:serversettings/game_mode` and returns a
    /// [`crate::GameMode`] decoded from the protocol's lowercase string.
    pub async fn game_mode(&self) -> Result<GameMode> {
        self.get("game_mode", "mode").await
    }

    /// Sets the dedicated server's default game mode.
    ///
    /// Sends `minecraft:serversettings/game_mode/set` with `{ "mode": ... }`
    /// and returns the acknowledged [`crate::GameMode`].
    pub async fn set_game_mode(&self, mode: GameMode) -> Result<GameMode> {
        self.set("game_mode", "mode", mode, "mode").await
    }

    /// Returns the view distance in chunks.
    ///
    /// Sends `minecraft:serversettings/view_distance`. The result controls how
    /// far chunk data can be sent to players.
    pub async fn view_distance(&self) -> Result<i32> {
        self.get("view_distance", "distance").await
    }

    /// Sets the view distance in chunks.
    ///
    /// Sends `minecraft:serversettings/view_distance/set` with
    /// `{ "distance": ... }` and returns the accepted value.
    pub async fn set_view_distance(&self, distance: i32) -> Result<i32> {
        self.set("view_distance", "distance", distance, "distance")
            .await
    }

    /// Returns the simulation distance in chunks.
    ///
    /// Sends `minecraft:serversettings/simulation_distance`. This is distinct
    /// from view distance and controls the area in which game simulation runs.
    pub async fn simulation_distance(&self) -> Result<i32> {
        self.get("simulation_distance", "distance").await
    }

    /// Sets the simulation distance in chunks.
    ///
    /// Sends `minecraft:serversettings/simulation_distance/set` with
    /// `{ "distance": ... }` and returns the accepted value.
    pub async fn set_simulation_distance(&self, distance: i32) -> Result<i32> {
        self.set("simulation_distance", "distance", distance, "distance")
            .await
    }

    /// Returns whether the server accepts incoming player transfers.
    ///
    /// Sends `minecraft:serversettings/accept_transfers` and decodes `accepted`.
    pub async fn accept_transfers(&self) -> Result<bool> {
        self.get("accept_transfers", "accepted").await
    }

    /// Enables or disables acceptance of incoming player transfers.
    ///
    /// Sends `minecraft:serversettings/accept_transfers/set` with
    /// `{ "accept": ... }` and returns `accepted`.
    pub async fn set_accept_transfers(&self, accepted: bool) -> Result<bool> {
        self.set("accept_transfers", "accept", accepted, "accepted")
            .await
    }

    /// Returns the interval between server-status heartbeat notifications.
    ///
    /// Sends `minecraft:serversettings/status_heartbeat_interval`. A consumer
    /// can subscribe to `Event::ServerStatus` to receive these heartbeats.
    pub async fn status_heartbeat_interval(&self) -> Result<i32> {
        self.get("status_heartbeat_interval", "seconds").await
    }

    /// Sets the interval between server-status heartbeat notifications.
    ///
    /// Sends `minecraft:serversettings/status_heartbeat_interval/set` with
    /// `{ "seconds": ... }` and returns the server acknowledgement.
    pub async fn set_status_heartbeat_interval(&self, seconds: i32) -> Result<i32> {
        self.set("status_heartbeat_interval", "seconds", seconds, "seconds")
            .await
    }

    /// Returns the permission level required for operator commands.
    ///
    /// Sends `minecraft:serversettings/operator_user_permission_level` and
    /// decodes its `level` field.
    pub async fn operator_user_permission_level(&self) -> Result<i32> {
        self.get("operator_user_permission_level", "level").await
    }

    /// Sets the permission level required for operator commands.
    ///
    /// Sends `minecraft:serversettings/operator_user_permission_level/set`
    /// with `{ "level": ... }`. Minecraft validates the supplied level.
    pub async fn set_operator_user_permission_level(&self, level: i32) -> Result<i32> {
        self.set("operator_user_permission_level", "level", level, "level")
            .await
    }

    /// Returns whether online-player details are hidden in status queries.
    ///
    /// Sends `minecraft:serversettings/hide_online_players` and decodes `hidden`.
    pub async fn hide_online_players(&self) -> Result<bool> {
        self.get("hide_online_players", "hidden").await
    }

    /// Sets whether online-player details are hidden in status queries.
    ///
    /// Sends `minecraft:serversettings/hide_online_players/set` with
    /// `{ "hide": ... }` and returns `hidden`.
    pub async fn set_hide_online_players(&self, hidden: bool) -> Result<bool> {
        self.set("hide_online_players", "hide", hidden, "hidden")
            .await
    }

    /// Returns whether the server replies to connection status requests.
    ///
    /// Sends `minecraft:serversettings/status_replies` and decodes `enabled`.
    pub async fn status_replies(&self) -> Result<bool> {
        self.get("status_replies", "enabled").await
    }

    /// Enables or disables connection status replies.
    ///
    /// Sends `minecraft:serversettings/status_replies/set` with
    /// `{ "enable": ... }` and returns `enabled`.
    pub async fn set_status_replies(&self, enabled: bool) -> Result<bool> {
        self.set("status_replies", "enable", enabled, "enabled")
            .await
    }

    /// Returns the entity broadcast range in percentage points.
    ///
    /// Sends `minecraft:serversettings/entity_broadcast_range`. For example,
    /// `100` represents the normal configured range, not a fractional value.
    pub async fn entity_broadcast_range(&self) -> Result<i32> {
        self.get("entity_broadcast_range", "percentage_points")
            .await
    }

    /// Sets the entity broadcast range in percentage points.
    ///
    /// Sends `minecraft:serversettings/entity_broadcast_range/set` with
    /// `{ "percentage_points": ... }` and returns the accepted number.
    pub async fn set_entity_broadcast_range(&self, percentage_points: i32) -> Result<i32> {
        self.set(
            "entity_broadcast_range",
            "percentage_points",
            percentage_points,
            "percentage_points",
        )
        .await
    }

    async fn get<T>(&self, path: &str, result_field: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let result = self
            .client
            .call_typed_value(&format!("{ROOT}/{path}"), None)
            .await?;
        decode_field(result, result_field)
    }

    async fn set<P, T>(
        &self,
        path: &str,
        parameter_field: &str,
        parameter: P,
        result_field: &str,
    ) -> Result<T>
    where
        P: Serialize,
        T: DeserializeOwned,
    {
        let mut object = Map::new();
        object.insert(
            parameter_field.to_owned(),
            serde_json::to_value(parameter)
                .map_err(|error| Error::Serialization(error.to_string()))?,
        );
        let result = self
            .client
            .call_typed_value(&format!("{ROOT}/{path}/set"), Some(Value::Object(object)))
            .await?;
        decode_field(result, result_field)
    }
}

fn decode_field<T>(result: Value, field: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    let value = result
        .as_object()
        .and_then(|object| object.get(field))
        .cloned()
        .ok_or_else(|| Error::Deserialization(format!("MCSMP result is missing `{field}`")))?;
    serde_json::from_value(value).map_err(|error| Error::Deserialization(error.to_string()))
}
