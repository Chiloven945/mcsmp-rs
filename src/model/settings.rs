use serde::{Deserialize, Serialize};

/// Minecraft's server difficulty setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Difficulty {
    /// Peaceful difficulty.
    Peaceful,
    /// Easy difficulty.
    Easy,
    /// Normal difficulty.
    Normal,
    /// Hard difficulty.
    Hard,
}

/// Minecraft's default game-mode setting.
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
