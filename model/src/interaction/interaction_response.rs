use serde::{Serialize, Deserialize};
use serde_repr::{Serialize_repr, Deserialize_repr};
use crate::interaction::InteractionApplicationCommandCallbackData;

#[derive(Serialize, Deserialize, Debug)]
pub struct InteractionResponse {
    pub r#type: InteractionResponseType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<InteractionApplicationCommandCallbackData>,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy)]
#[repr(u8)]
pub enum InteractionResponseType {
    Pong = 1,
    ChannelMessageWithSource = 4,
    DeferredChannelMessageWithSource = 5,
}

impl InteractionResponse {
    pub fn new_pong() -> InteractionResponse {
        InteractionResponse {
            r#type: InteractionResponseType::Pong,
            data: None
        }
    }

    pub fn new_deferred_message_with_source() -> InteractionResponse {
        InteractionResponse {
            r#type: InteractionResponseType::DeferredChannelMessageWithSource,
            data: None
        }
    }
}