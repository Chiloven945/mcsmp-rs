use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{Client, Difficulty, Error, GameMode, Result};

const ROOT: &str = "minecraft:serversettings";

/// Strongly typed access to live dedicated-server settings.
///
/// Setters return the value accepted by the server. The service intentionally
/// leaves range validation to the server because MCSMP does not define all
/// numeric limits client-side.
#[derive(Clone, Debug)]
pub struct ServerSettingsApi {
    client: Client,
}

impl ServerSettingsApi {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Gets whether automatic world saving is enabled.
    pub async fn autosave(&self) -> Result<bool> {
        self.get("autosave", "enabled").await
    }

    /// Enables or disables automatic world saving.
    pub async fn set_autosave(&self, enabled: bool) -> Result<bool> {
        self.set("autosave", "enable", enabled, "enabled").await
    }

    /// Gets the current difficulty.
    pub async fn difficulty(&self) -> Result<Difficulty> {
        self.get("difficulty", "difficulty").await
    }

    /// Sets the current difficulty.
    pub async fn set_difficulty(&self, difficulty: Difficulty) -> Result<Difficulty> {
        self.set("difficulty", "difficulty", difficulty, "difficulty")
            .await
    }

    /// Gets whether removal from the allowlist is immediately enforced.
    pub async fn enforce_allowlist(&self) -> Result<bool> {
        self.get("enforce_allowlist", "enforced").await
    }

    /// Enables or disables immediate allowlist enforcement.
    pub async fn set_enforce_allowlist(&self, enforced: bool) -> Result<bool> {
        self.set("enforce_allowlist", "enforce", enforced, "enforced")
            .await
    }

    /// Gets whether the allowlist controls which players may join.
    pub async fn use_allowlist(&self) -> Result<bool> {
        self.get("use_allowlist", "used").await
    }

    /// Enables or disables allowlist use for joining players.
    pub async fn set_use_allowlist(&self, used: bool) -> Result<bool> {
        self.set("use_allowlist", "use", used, "used").await
    }

    /// Gets the maximum number of players.
    pub async fn max_players(&self) -> Result<i32> {
        self.get("max_players", "max").await
    }

    /// Sets the maximum number of players.
    pub async fn set_max_players(&self, max_players: i32) -> Result<i32> {
        self.set("max_players", "max", max_players, "max").await
    }

    /// Gets the empty-server pause delay in seconds.
    pub async fn pause_when_empty_seconds(&self) -> Result<i32> {
        self.get("pause_when_empty_seconds", "seconds").await
    }

    /// Sets the empty-server pause delay in seconds.
    pub async fn set_pause_when_empty_seconds(&self, seconds: i32) -> Result<i32> {
        self.set("pause_when_empty_seconds", "seconds", seconds, "seconds")
            .await
    }

    /// Gets the idle-player kick timeout in seconds.
    pub async fn player_idle_timeout(&self) -> Result<i32> {
        self.get("player_idle_timeout", "seconds").await
    }

    /// Sets the idle-player kick timeout in seconds.
    pub async fn set_player_idle_timeout(&self, seconds: i32) -> Result<i32> {
        self.set("player_idle_timeout", "seconds", seconds, "seconds")
            .await
    }

    /// Gets whether Survival-mode flight is allowed.
    pub async fn allow_flight(&self) -> Result<bool> {
        self.get("allow_flight", "allowed").await
    }

    /// Enables or disables Survival-mode flight.
    pub async fn set_allow_flight(&self, allowed: bool) -> Result<bool> {
        self.set("allow_flight", "allowed", allowed, "allowed")
            .await
    }

    /// Gets the server message of the day.
    pub async fn motd(&self) -> Result<String> {
        self.get("motd", "message").await
    }

    /// Sets the server message of the day.
    pub async fn set_motd(&self, message: impl Into<String>) -> Result<String> {
        self.set("motd", "message", message.into(), "message").await
    }

    /// Gets the spawn-protection radius in blocks.
    pub async fn spawn_protection_radius(&self) -> Result<i32> {
        self.get("spawn_protection_radius", "radius").await
    }

    /// Sets the spawn-protection radius in blocks.
    pub async fn set_spawn_protection_radius(&self, radius: i32) -> Result<i32> {
        self.set("spawn_protection_radius", "radius", radius, "radius")
            .await
    }

    /// Gets whether players are forced into the default game mode.
    pub async fn force_game_mode(&self) -> Result<bool> {
        self.get("force_game_mode", "forced").await
    }

    /// Enables or disables forcing the default game mode.
    pub async fn set_force_game_mode(&self, forced: bool) -> Result<bool> {
        self.set("force_game_mode", "force", forced, "forced").await
    }

    /// Gets the default game mode.
    pub async fn game_mode(&self) -> Result<GameMode> {
        self.get("game_mode", "mode").await
    }

    /// Sets the default game mode.
    pub async fn set_game_mode(&self, mode: GameMode) -> Result<GameMode> {
        self.set("game_mode", "mode", mode, "mode").await
    }

    /// Gets the view distance in chunks.
    pub async fn view_distance(&self) -> Result<i32> {
        self.get("view_distance", "distance").await
    }

    /// Sets the view distance in chunks.
    pub async fn set_view_distance(&self, distance: i32) -> Result<i32> {
        self.set("view_distance", "distance", distance, "distance")
            .await
    }

    /// Gets the simulation distance in chunks.
    pub async fn simulation_distance(&self) -> Result<i32> {
        self.get("simulation_distance", "distance").await
    }

    /// Sets the simulation distance in chunks.
    pub async fn set_simulation_distance(&self, distance: i32) -> Result<i32> {
        self.set("simulation_distance", "distance", distance, "distance")
            .await
    }

    /// Gets whether the server accepts inter-server player transfers.
    pub async fn accept_transfers(&self) -> Result<bool> {
        self.get("accept_transfers", "accepted").await
    }

    /// Enables or disables acceptance of inter-server player transfers.
    pub async fn set_accept_transfers(&self, accepted: bool) -> Result<bool> {
        self.set("accept_transfers", "accept", accepted, "accepted")
            .await
    }

    /// Gets the interval between server status heartbeats in seconds.
    pub async fn status_heartbeat_interval(&self) -> Result<i32> {
        self.get("status_heartbeat_interval", "seconds").await
    }

    /// Sets the interval between server status heartbeats in seconds.
    pub async fn set_status_heartbeat_interval(&self, seconds: i32) -> Result<i32> {
        self.set("status_heartbeat_interval", "seconds", seconds, "seconds")
            .await
    }

    /// Gets the operator command permission level.
    pub async fn operator_user_permission_level(&self) -> Result<i32> {
        self.get("operator_user_permission_level", "level").await
    }

    /// Sets the operator command permission level.
    pub async fn set_operator_user_permission_level(&self, level: i32) -> Result<i32> {
        self.set("operator_user_permission_level", "level", level, "level")
            .await
    }

    /// Gets whether online players are hidden from status queries.
    pub async fn hide_online_players(&self) -> Result<bool> {
        self.get("hide_online_players", "hidden").await
    }

    /// Sets whether online players are hidden from status queries.
    pub async fn set_hide_online_players(&self, hidden: bool) -> Result<bool> {
        self.set("hide_online_players", "hide", hidden, "hidden")
            .await
    }

    /// Gets whether the server responds to connection-status requests.
    pub async fn status_replies(&self) -> Result<bool> {
        self.get("status_replies", "enabled").await
    }

    /// Enables or disables connection-status replies.
    pub async fn set_status_replies(&self, enabled: bool) -> Result<bool> {
        self.set("status_replies", "enable", enabled, "enabled")
            .await
    }

    /// Gets the entity broadcast range in percentage points.
    pub async fn entity_broadcast_range(&self) -> Result<i32> {
        self.get("entity_broadcast_range", "percentage_points")
            .await
    }

    /// Sets the entity broadcast range in percentage points.
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
