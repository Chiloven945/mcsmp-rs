use serde::{Deserialize, Serialize};

use crate::api::{call, params};
use crate::{Client, Result, TypedGameRule, UntypedGameRule};

const ROOT: &str = "minecraft:gamerules";

/// Strongly typed access to `minecraft:gamerules` operations.
#[derive(Clone, Debug)]
pub struct GamerulesApi {
    client: Client,
}

impl GamerulesApi {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Gets all available gamerules with their server-declared types and values.
    pub async fn list(&self) -> Result<Vec<TypedGameRule>> {
        Ok(call::<GamerulesResult>(&self.client, ROOT, None)
            .await?
            .gamerules)
    }

    /// Updates a gamerule and returns its typed server-side value.
    pub async fn update(&self, gamerule: UntypedGameRule) -> Result<TypedGameRule> {
        Ok(call::<GameruleResult>(
            &self.client,
            "minecraft:gamerules/update",
            Some(params(UpdateParams { gamerule })?),
        )
        .await?
        .gamerule)
    }
}

#[derive(Deserialize)]
struct GamerulesResult {
    gamerules: Vec<TypedGameRule>,
}

#[derive(Deserialize)]
struct GameruleResult {
    gamerule: TypedGameRule,
}

#[derive(Serialize)]
struct UpdateParams {
    gamerule: UntypedGameRule,
}
