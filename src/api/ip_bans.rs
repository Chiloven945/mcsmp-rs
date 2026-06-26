use std::net::IpAddr;

use serde::{Deserialize, Serialize};

use crate::api::{call, params};
use crate::{Client, IncomingIpBan, IpBan, Result};

const ROOT: &str = "minecraft:ip_bans";

/// Strongly typed access to `minecraft:ip_bans` operations.
#[derive(Clone, Debug)]
pub struct IpBansApi {
    client: Client,
}

impl IpBansApi {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Gets the current resolved IP-ban list.
    pub async fn list(&self) -> Result<Vec<IpBan>> {
        Ok(call::<BanListResult>(&self.client, ROOT, None)
            .await?
            .banlist)
    }

    /// Replaces the IP-ban list and returns the resulting server snapshot.
    pub async fn set(&self, bans: impl IntoIterator<Item = IpBan>) -> Result<Vec<IpBan>> {
        let banlist: Vec<_> = bans.into_iter().collect();
        Ok(call::<BanListResult>(
            &self.client,
            "minecraft:ip_bans/set",
            Some(params(BanListParams { banlist })?),
        )
        .await?
        .banlist)
    }

    /// Adds direct or player-resolved IP bans and returns the resolved server
    /// snapshot.
    pub async fn add(&self, bans: impl IntoIterator<Item = IncomingIpBan>) -> Result<Vec<IpBan>> {
        let add: Vec<_> = bans.into_iter().collect();
        Ok(call::<BanListResult>(
            &self.client,
            "minecraft:ip_bans/add",
            Some(params(AddParams { add })?),
        )
        .await?
        .banlist)
    }

    /// Removes bans for the supplied direct IP addresses and returns the
    /// resulting server snapshot.
    pub async fn remove(&self, addresses: impl IntoIterator<Item = IpAddr>) -> Result<Vec<IpBan>> {
        let ip: Vec<_> = addresses.into_iter().collect();
        Ok(call::<BanListResult>(
            &self.client,
            "minecraft:ip_bans/remove",
            Some(params(RemoveParams { ip })?),
        )
        .await?
        .banlist)
    }

    /// Clears all IP bans and returns the resulting server snapshot.
    pub async fn clear(&self) -> Result<Vec<IpBan>> {
        Ok(
            call::<BanListResult>(&self.client, "minecraft:ip_bans/clear", None)
                .await?
                .banlist,
        )
    }
}

#[derive(Deserialize)]
struct BanListResult {
    banlist: Vec<IpBan>,
}

#[derive(Serialize)]
struct BanListParams {
    banlist: Vec<IpBan>,
}

#[derive(Serialize)]
struct AddParams {
    add: Vec<IncomingIpBan>,
}

#[derive(Serialize)]
struct RemoveParams {
    ip: Vec<IpAddr>,
}
