//! Strongly typed MCSMP request and response models.
//!
//! Models in this module describe stable MCSMP resources, including server
//! administration data, live settings, and gamerules across legacy and current
//! protocol representations.

mod ban;
mod gamerule;
mod message;
mod player;
mod server;
mod settings;

pub use ban::{IncomingIpBan, IpBan, UserBan};
pub use gamerule::{GameRuleKind, GameRuleValue, TypedGameRule, UntypedGameRule};
pub use message::{KickPlayer, Message, SystemMessage};
pub use player::{ModelError, PlayerRef};
pub use server::{MinecraftVersion, Operator, ServerState};
pub use settings::{Difficulty, GameMode};
