use serde::{Serialize, Deserialize};
use serde_repr::{Serialize_repr, Deserialize_repr};
use crate::Snowflake;
use crate::guild::Member;
use crate::interaction::ApplicationCommandInteractionData;

#[derive(Serialize, Deserialize, Debug)]
pub struct Interaction {
    pub id: Snowflake,
    pub r#type: InteractionType,
    pub data: Option<ApplicationCommandInteractionData>,
    pub guild_id: Snowflake,
    pub channel_id: Snowflake,
    pub member: Member,
    pub token: Box<str>,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy)]
#[repr(u8)]
pub enum InteractionType {
    Ping = 1,
    ApplicationCommand = 2,
}