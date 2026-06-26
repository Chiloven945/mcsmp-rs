//! Typed operations for the official `minecraft:gamerules` namespace.

use serde::{Deserialize, Serialize};

use crate::api::{call, params};
use crate::{Client, Result, TypedGameRule, UntypedGameRule};

const ROOT: &str = "minecraft:gamerules";

/// Typed handle for querying and changing gamerules.
///
/// Obtain this handle from [`crate::Client::gamerules`]. The protocol separates
/// a server-returned [`TypedGameRule`] (which includes a declared kind) from
/// an [`UntypedGameRule`] update request (which contains only a key and scalar
/// value). This mirrors MCSMP's wire contract.
#[derive(Clone, Debug)]
pub struct GamerulesApi {
    client: Client,
}

impl GamerulesApi {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Retrieves every available gamerule with its declared kind and value.
    ///
    /// This maps to `minecraft:gamerules`. Current MCSMP servers return native
    /// JSON booleans and signed integers. Older servers may return strings,
    /// preserved as [`crate::GameRuleValue::LegacyString`] without automatic
    /// coercion.
    pub async fn list(&self) -> Result<Vec<TypedGameRule>> {
        Ok(call::<GamerulesResult>(&self.client, ROOT, None)
            .await?
            .gamerules)
    }

    /// Updates one gamerule and returns the server's typed value.
    ///
    /// This maps to `minecraft:gamerules/update`. Construct `gamerule` with
    /// `UntypedGameRule::boolean`, `UntypedGameRule::integer`, or
    /// `UntypedGameRule::legacy_string`. The response is validated against the
    /// server-declared kind; a malformed response such as
    /// `type: "integer"` plus `value: false` returns
    /// [`crate::Error::Deserialization`] instead of silently accepting an
    /// invalid state.
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
