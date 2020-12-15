use serde::{Serialize, Deserialize};
use serde_repr::{Serialize_repr, Deserialize_repr};
use crate::interaction::ApplicationCommandOptionChoice;

#[derive(Serialize, Deserialize, Debug)]
pub struct ApplicationCommandOption {
    pub r#type: ApplicationCommandOptionType,
    pub name: Box<str>,
    pub description: Box<str>,
    pub default: bool,
    pub required: bool,
    pub choices: Vec<ApplicationCommandOptionChoice>,
    pub options: Option<Vec<ApplicationCommandOption>>,
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
}