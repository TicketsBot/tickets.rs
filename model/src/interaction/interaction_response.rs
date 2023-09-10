use crate::interaction::{
    ApplicationCommandOptionChoice, Component, InteractionApplicationCommandCallbackData,
};
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use serde_repr::{Deserialize_repr, Serialize_repr};

// TODO: Reduce redundant code

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum InteractionResponse {
    PongResponse(SimpleInteractionResponse),
    ChannelMessageWithSource(ApplicationCommandResponse),
    DeferredChannelMessageWithSource(DeferredApplicationCommandResponse),
    DeferredMessageUpdate(SimpleInteractionResponse),
    ApplicationCommandAutoCompleteResult(ApplicationCommandAutoCompleteResultResponse),
    Modal(ModalResponse),
    // UpdateMessage is not yet supported
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SimpleInteractionResponse {
    r#type: InteractionResponseType,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApplicationCommandResponse {
    r#type: InteractionResponseType,
    data: InteractionApplicationCommandCallbackData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeferredApplicationCommandResponse {
    r#type: InteractionResponseType,
    data: DeferredApplicationCommandResponseData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeferredApplicationCommandResponseData {
    pub flags: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApplicationCommandAutoCompleteResultResponse {
    r#type: InteractionResponseType,
    data: ApplicationCommandAutoCompleteResultResponseData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApplicationCommandAutoCompleteResultResponseData {
    pub choices: Vec<ApplicationCommandOptionChoice>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ModalResponse {
    r#type: InteractionResponseType,
    data: ModalResponseData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ModalResponseData {
    pub custom_id: Box<str>,
    pub title: Box<str>,
    pub components: Vec<Component>,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy)]
#[repr(u8)]
#[non_exhaustive]
pub enum InteractionResponseType {
    Pong = 1,
    ChannelMessageWithSource = 4,
    DeferredChannelMessageWithSource = 5,
    DeferredMessageUpdate = 6,
    UpdateMessage = 7,
    ApplicationCommandAutoCompleteResult = 8,
    Modal = 9,
}

impl TryFrom<u64> for InteractionResponseType {
    type Error = Box<str>;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Ok(match value {
            1 => Self::Pong,
            4 => Self::ChannelMessageWithSource,
            5 => Self::DeferredChannelMessageWithSource,
            6 => Self::DeferredMessageUpdate,
            7 => Self::UpdateMessage,
            8 => Self::ApplicationCommandAutoCompleteResult,
            9 => Self::Modal,
            _ => {
                return Err(
                    format!("invalid interaction response type \"{}\"", value).into_boxed_str()
                )
            }
        })
    }
}

impl InteractionResponse {
    pub fn new_pong() -> InteractionResponse {
        InteractionResponse::PongResponse(SimpleInteractionResponse {
            r#type: InteractionResponseType::Pong,
        })
    }

    pub fn new_channel_message_with_source(
        data: InteractionApplicationCommandCallbackData,
    ) -> InteractionResponse {
        InteractionResponse::ChannelMessageWithSource(ApplicationCommandResponse {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data,
        })
    }

    pub fn new_deferred_message_with_source() -> InteractionResponse {
        InteractionResponse::DeferredChannelMessageWithSource(DeferredApplicationCommandResponse {
            r#type: InteractionResponseType::DeferredChannelMessageWithSource,
            data: DeferredApplicationCommandResponseData { flags: 64 },
        })
    }

    pub fn new_deferred_message_update() -> InteractionResponse {
        InteractionResponse::DeferredMessageUpdate(SimpleInteractionResponse {
            r#type: InteractionResponseType::DeferredMessageUpdate,
        })
    }

    pub fn new_application_command_auto_complete_result_response(
        choices: Vec<ApplicationCommandOptionChoice>,
    ) -> InteractionResponse {
        InteractionResponse::ApplicationCommandAutoCompleteResult(
            ApplicationCommandAutoCompleteResultResponse {
                r#type: InteractionResponseType::ApplicationCommandAutoCompleteResult,
                data: ApplicationCommandAutoCompleteResultResponseData { choices },
            },
        )
    }
}

impl<'de> Deserialize<'de> for InteractionResponse {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;

        let response_type = value
            .get("type")
            .and_then(Value::as_u64)
            .ok_or_else(|| Box::from("interaction response type was not an integer"))
            .and_then(InteractionResponseType::try_from)
            .map_err(D::Error::custom)?;

        let response = match response_type {
            InteractionResponseType::Pong => {
                serde_json::from_value(value).map(InteractionResponse::PongResponse)
            }
            InteractionResponseType::ChannelMessageWithSource => {
                serde_json::from_value(value).map(InteractionResponse::ChannelMessageWithSource)
            }
            InteractionResponseType::DeferredChannelMessageWithSource => {
                serde_json::from_value(value)
                    .map(InteractionResponse::DeferredChannelMessageWithSource)
            }
            InteractionResponseType::DeferredMessageUpdate => {
                serde_json::from_value(value).map(InteractionResponse::DeferredMessageUpdate)
            }
            InteractionResponseType::UpdateMessage => {
                Err(Error::custom("UpdateMessage is not yet supported"))
            }
            InteractionResponseType::ApplicationCommandAutoCompleteResult => {
                serde_json::from_value(value)
                    .map(InteractionResponse::ApplicationCommandAutoCompleteResult)
            }
            InteractionResponseType::Modal => {
                serde_json::from_value(value).map(InteractionResponse::Modal)
            }
        }
        .map_err(D::Error::custom)?;

        Ok(response)
    }
}
