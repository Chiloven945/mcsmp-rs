//! Typed operations for the official `minecraft:ip_bans` namespace.
//!
//! The resource distinguishes a resolved [`crate::IpBan`] from an
//! [`crate::IncomingIpBan`] request. The latter can ask Minecraft to resolve a
//! player's current address, while the former always contains a concrete IP.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

use crate::api::{call, params};
use crate::{Client, IncomingIpBan, IpBan, Result};

const ROOT: &str = "minecraft:ip_bans";

/// Typed handle for the official IP-ban resource.
///
/// Obtain this handle from [`crate::Client::ip_bans`]. All mutations return
/// the server's full, resolved IP-ban list after the change.
#[derive(Clone, Debug)]
pub struct IpBansApi {
    client: Client,
}

impl IpBansApi {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Retrieves the resolved IP-ban list with `minecraft:ip_bans`.
    ///
    /// Returned entries always contain an actual [`std::net::IpAddr`]. A
    /// player selector used while creating a ban is not retained as the ban's
    /// primary identity in this result.
    pub async fn list(&self) -> Result<Vec<IpBan>> {
        Ok(call::<BanListResult>(&self.client, ROOT, None)
            .await?
            .banlist)
    }

    /// Replaces the entire resolved IP-ban list with `bans`.
    ///
    /// This maps to `minecraft:ip_bans/set`. Passing an empty iterator clears
    /// all IP bans. Because the request accepts concrete addresses only, use
    /// [`Self::add`] when Minecraft should resolve an address from a player
    /// selector.
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

    /// Adds IP bans from direct addresses, player selectors, or both.
    ///
    /// This maps to `minecraft:ip_bans/add`. An [`IncomingIpBan`] created with
    /// `IncomingIpBan::player` asks the server to resolve the selected
    /// player's address. The returned [`IpBan`] values show the concrete
    /// addresses that Minecraft accepted.
    ///
    /// Banning may disconnect active players. Do not automatically repeat this
    /// operation after a transport interruption.
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

    /// Removes IP bans for the supplied concrete addresses.
    ///
    /// This maps to `minecraft:ip_bans/remove`. The protocol expects IP
    /// strings, represented here by [`std::net::IpAddr`], rather than player
    /// selectors. Returns the full list after removal.
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

    /// Clears every IP ban with `minecraft:ip_bans/clear`.
    ///
    /// Returns the resulting full list, normally empty.
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
