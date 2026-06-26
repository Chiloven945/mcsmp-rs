//! Strongly typed MCSMP request and response models.
//!
//! Models in this module describe the stable resources used by the Milestone 2
//! official API groups. Later protocol additions, such as game rules, are
//! intentionally introduced in their corresponding implementation milestone.

mod ban;
mod message;
mod player;
mod server;

pub use ban::{IncomingIpBan, IpBan, UserBan};
pub use message::{KickPlayer, Message, SystemMessage};
pub use player::{ModelError, PlayerRef};
pub use server::{MinecraftVersion, Operator, ServerState};
