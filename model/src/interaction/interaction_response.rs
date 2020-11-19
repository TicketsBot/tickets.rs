use serde::{Serialize, Deserialize};
use serde_repr::{Serialize_repr, Deserialize_repr};
use crate::interaction::InteractionApplicationCommandCallbackData;

#[derive(Serialize, Deserialize, Debug)]
pub struct InteractionResponse {
    pub r#type: InteractionResponseType,
    pub data: Option<InteractionApplicationCommandCallbackData>,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy)]
#[repr(u8)]
pub enum InteractionResponseType {
    Pong = 1,
    Acknowledge = 2,
    ChannelMessage = 3,
    ChannelMessageWithSource = 4,
}