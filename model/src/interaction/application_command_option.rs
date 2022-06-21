use crate::channel::ChannelType;
use crate::interaction::ApplicationCommandOptionChoice;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Serialize, Deserialize, Debug)]
pub struct ApplicationCommandOption {
    pub r#type: ApplicationCommandOptionType,
    pub name: Box<str>,
    pub description: Box<str>,
    pub default: bool,
    pub required: bool,
    pub choices: Vec<ApplicationCommandOptionChoice>,
    #[serde(default)]
    pub autocomplete: bool,
    pub options: Option<Vec<ApplicationCommandOption>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_types: Option<Vec<ChannelType>>,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy)]
#[repr(u8)]
pub enum ApplicationCommandOptionType {
    SubCommand = 1,
    SubCommandGroup = 2,
    String = 3,
    Integer = 4,
    Boolean = 5,
    User = 6,
    Channel = 7,
    Role = 8,
    Mentionable = 9,
    Number = 10,
    Attachment = 11,
}
