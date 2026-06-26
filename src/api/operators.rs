use serde::{Deserialize, Serialize};

use crate::api::{call, params};
use crate::{Client, Operator, PlayerRef, Result};

const ROOT: &str = "minecraft:operators";

/// Strongly typed access to `minecraft:operators` operations.
#[derive(Clone, Debug)]
pub struct OperatorsApi {
    client: Client,
}

impl OperatorsApi {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Gets the current operator list.
    pub async fn list(&self) -> Result<Vec<Operator>> {
        Ok(call::<OperatorsResult>(&self.client, ROOT, None)
            .await?
            .operators)
    }

    /// Replaces the operator list and returns the resulting server snapshot.
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

    /// Adds operators and returns the resulting server snapshot.
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

    /// Removes operator privileges for the selected players and returns the
    /// resulting server snapshot.
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

    /// Removes all operators and returns the resulting server snapshot.
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
