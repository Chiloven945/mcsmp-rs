use serde::{Deserialize, Serialize};

/// Minecraft difficulty setting used by the live server-settings API.
///
/// The protocol serializes these values as lowercase strings. The enum is
/// `non_exhaustive` so downstream code should include a fallback arm when
/// matching future protocol additions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Difficulty {
    /// Peaceful difficulty, with hostile gameplay threats disabled by Minecraft.
    Peaceful,
    /// Easy difficulty, with reduced hostile-mob damage compared with normal.
    Easy,
    /// Normal difficulty, Minecraft's standard survival balance.
    Normal,
    /// Hard difficulty, with Minecraft's stricter hostile-mob behavior.
    Hard,
}

/// Default Minecraft game mode used by the live server-settings API.
///
/// The protocol serializes these values as lowercase strings. This setting is
/// distinct from `force_game_mode`, which controls whether players are forced
/// to adopt the default when they join.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum GameMode {
    /// Survival mode.
    Survival,
    /// Creative mode.
    Creative,
    /// Adventure mode.
    Adventure,
    /// Spectator mode.
    Spectator,
}
