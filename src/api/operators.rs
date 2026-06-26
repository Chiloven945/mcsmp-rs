//! Typed operations for the official `minecraft:operators` namespace.

use serde::{Deserialize, Serialize};

use crate::api::{call, params};
use crate::{Client, Operator, PlayerRef, Result};

const ROOT: &str = "minecraft:operators";

/// Typed handle for the official operator-list resource.
///
/// Obtain this handle from [`crate::Client::operators`]. Minecraft's operator
/// records can include an optional command permission level and player-limit
/// bypass flag; represent those values with [`crate::Operator`].
#[derive(Clone, Debug)]
pub struct OperatorsApi {
    client: Client,
}

impl OperatorsApi {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Retrieves every current operator entry with `minecraft:operators`.
    ///
    /// The returned snapshot includes server-defaulted values when the server
    /// exposes them. It is more authoritative than the inputs used for prior
    /// add or set calls.
    pub async fn list(&self) -> Result<Vec<Operator>> {
        Ok(call::<OperatorsResult>(&self.client, ROOT, None)
            .await?
            .operators)
    }

    /// Replaces the entire operator list with `operators`.
    ///
    /// This maps to `minecraft:operators/set`. An empty iterator removes every
    /// operator. Prefer [`Self::add`] and [`Self::remove`] for incremental
    /// administration, because this method is destructive.
    pub async fn set(
        &self,
        operators: impl IntoIterator<Item = Operator>,
    ) -> Result<Vec<Operator>> {
        let operators: Vec<_> = operators.into_iter().collect();
        Ok(call::<OperatorsResult>(
            &self.client,
            "minecraft:operators/set",
            Some(params(OperatorsParams { operators })?),
        )
        .await?
        .operators)
    }

    /// Adds operator entries to the server.
    ///
    /// This maps to `minecraft:operators/add`. Use
    /// [`crate::Operator::with_permission_level`] or
    /// [`crate::Operator::with_options`] to validate the protocol's `0..=4`
    /// permission range before sending the request.
    pub async fn add(
        &self,
        operators: impl IntoIterator<Item = Operator>,
    ) -> Result<Vec<Operator>> {
        let add: Vec<_> = operators.into_iter().collect();
        Ok(call::<OperatorsResult>(
            &self.client,
            "minecraft:operators/add",
            Some(params(AddParams { add })?),
        )
        .await?
        .operators)
    }

    /// Removes operator privileges for selected players.
    ///
    /// This maps to `minecraft:operators/remove`. Each [`PlayerRef`] can
    /// identify a player by UUID, name, or both. The returned value is the full
    /// operator list after removal.
    pub async fn remove(
        &self,
        players: impl IntoIterator<Item = PlayerRef>,
    ) -> Result<Vec<Operator>> {
        let remove: Vec<_> = players.into_iter().collect();
        Ok(call::<OperatorsResult>(
            &self.client,
            "minecraft:operators/remove",
            Some(params(RemoveParams { remove })?),
        )
        .await?
        .operators)
    }

    /// Removes every operator entry with `minecraft:operators/clear`.
    ///
    /// Returns the resulting full list, normally empty.
    pub async fn clear(&self) -> Result<Vec<Operator>> {
        Ok(
            call::<OperatorsResult>(&self.client, "minecraft:operators/clear", None)
                .await?
                .operators,
        )
    }
}

#[derive(Deserialize)]
struct OperatorsResult {
    operators: Vec<Operator>,
}

#[derive(Serialize)]
struct OperatorsParams {
    operators: Vec<Operator>,
}

#[derive(Serialize)]
struct AddParams {
    add: Vec<Operator>,
}

#[derive(Serialize)]
struct RemoveParams {
    remove: Vec<PlayerRef>,
}
