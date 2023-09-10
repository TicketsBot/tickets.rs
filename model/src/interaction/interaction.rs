use crate::channel::message::Message;
use crate::guild::Member;
use crate::interaction::{
    ApplicationCommandInteractionData, ApplicationCommandInteractionDataOption,
    ApplicationCommandType, ComponentType,
};
use crate::user::User;
use crate::{PermissionBitSet, Snowflake};
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Serialize, Debug)]
#[serde(untagged)]
#[non_exhaustive]
pub enum Interaction {
    Ping(Box<PingInteraction>),
    ApplicationCommand(Box<ApplicationCommandInteraction>),
    MessageComponent(Box<MessageComponentInteraction>),
    ApplicationCommandAutoComplete(Box<ApplicationCommandAutoCompleteInteraction>),
    ModalSubmit(Box<ModalSubmitInteraction>),
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy)]
#[repr(u8)]
pub enum InteractionType {
    Ping = 1,
    ApplicationCommand = 2,
    MessageComponent = 3,
    ApplicationCommandAutoComplete = 4,
    ModalSubmit = 5,
}

impl TryFrom<u64> for InteractionType {
    type Error = Box<str>;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Ok(match value {
            1 => Self::Ping,
            2 => Self::ApplicationCommand,
            3 => Self::MessageComponent,
            4 => Self::ApplicationCommandAutoComplete,
            5 => Self::ModalSubmit,
            _ => return Err(format!("invalid interaction type \"{}\"", value).into_boxed_str()),
        })
    }
}

// ============================================================================
// Ping Interaction
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct PingInteraction {
    pub id: Snowflake,
    pub application_id: Snowflake,
    pub r#type: InteractionType,
}

// ============================================================================
// Application Command Interaction
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct ApplicationCommandInteraction {
    pub id: Snowflake,
    pub application_id: Snowflake,
    pub r#type: InteractionType,
    pub data: ApplicationCommandInteractionData,
    pub guild_id: Option<Snowflake>,
    pub channel_id: Snowflake,
    pub member: Option<Member>,
    pub user: Option<User>,
    pub token: Box<str>,
    pub version: u8,
    pub app_permissions: Option<PermissionBitSet>,
}

// ============================================================================
// Message Component Interaction
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageComponentInteraction {
    pub id: Snowflake,
    pub application_id: Snowflake,
    pub r#type: InteractionType,
    pub message: Message,
    pub data: MessageComponentInteractionData,
    pub guild_id: Option<Snowflake>,
    pub channel_id: Snowflake,
    pub member: Option<Member>,
    pub user: Option<User>,
    pub token: Box<str>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageComponentInteractionData {
    pub custom_id: Box<str>,
    pub component_type: ComponentType,
}

// ============================================================================
// Auto Complete Interaction
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct ApplicationCommandAutoCompleteInteraction {
    pub id: Snowflake,
    pub application_id: Snowflake,
    pub r#type: InteractionType,
    pub data: ApplicationCommandAutoCompleteInteractionData,
    pub guild_id: Option<Snowflake>,
    pub channel_id: Snowflake,
    pub member: Option<Member>,
    pub user: Option<User>,
    pub token: Box<str>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApplicationCommandAutoCompleteInteractionData {
    pub id: Snowflake,
    pub name: Box<str>,
    pub options: Vec<ApplicationCommandInteractionDataOption>,
    pub r#type: ApplicationCommandType,
}

// ============================================================================
// Modal Submit Interaction
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct ModalSubmitInteraction {
    pub id: Snowflake,
    pub application_id: Snowflake,
    pub r#type: InteractionType,
    pub message: Message,
    pub data: ModalInteractionData,
    pub guild_id: Option<Snowflake>,
    pub channel_id: Snowflake,
    pub member: Option<Member>,
    pub user: Option<User>,
    pub token: Box<str>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ModalInteractionData {
    pub custom_id: Box<str>,
    pub components: Vec<ModalInteractionActionRowData>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ModalInteractionActionRowData {
    pub r#type: ComponentType,
    pub components: Vec<ModalInteractionComponentData>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ModalInteractionComponentData {
    pub r#type: ComponentType,
    pub custom_id: Box<str>,
    pub value: Box<str>,
}

impl<'de> Deserialize<'de> for Interaction {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;

        let interaction_type = value
            .get("type")
            .and_then(Value::as_u64)
            .ok_or_else(|| Box::from("interaction type was not an integer"))
            .and_then(InteractionType::try_from)
            .map_err(D::Error::custom)?;

        let component = match interaction_type {
            InteractionType::Ping => serde_json::from_value(value).map(Interaction::Ping),
            InteractionType::ApplicationCommand => {
                serde_json::from_value(value).map(Interaction::ApplicationCommand)
            }
            InteractionType::MessageComponent => {
                serde_json::from_value(value).map(Interaction::MessageComponent)
            }
            InteractionType::ApplicationCommandAutoComplete => {
                serde_json::from_value(value).map(Interaction::ApplicationCommandAutoComplete)
            }
            InteractionType::ModalSubmit => {
                serde_json::from_value(value).map(Interaction::ModalSubmit)
            }
        }
        .map_err(D::Error::custom)?;

        Ok(component)
    }
}
