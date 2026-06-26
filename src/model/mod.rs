//! Strongly typed MCSMP request and response models.
//!
//! This module contains the values accepted by typed resource methods and
//! returned by official MCSMP responses and notifications. Constructors enforce
//! local invariants that are independent of the remote server, such as
//! non-blank selectors and valid operator permission levels. They intentionally
//! leave server-owned validation—account existence, current addresses,
//! timestamp interpretation, dynamic numeric limits, and persistence—to
//! Minecraft.
//!
//! Most models implement `serde::Serialize` and/or `serde::Deserialize` using
//! the exact camelCase or protocol-specific field names required on the wire.
//! `GameRuleValue` additionally preserves supported legacy scalar forms rather
//! than coercing them silently.

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
