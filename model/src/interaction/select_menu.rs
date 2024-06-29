use crate::{channel::ChannelType, guild::Emoji, Snowflake};
use serde::{Deserialize, Serialize};

use super::ComponentType;

#[derive(Serialize, Deserialize, Debug)]
pub struct SelectMenu {
    pub r#type: ComponentType,
    pub custom_id: Box<str>,
    pub options: Vec<SelectOption>,
    pub channel_types: Option<Vec<ChannelType>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<Box<str>>,
    /// 0-25
    #[serde(default = "one")]
    pub min_values: u8,
    /// 1-25
    #[serde(default = "one")]
    pub max_values: u8,
    #[serde(default = "Default::default")]
    pub disabled: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SelectOption {
    pub label: Box<str>,
    pub value: Box<str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Box<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<Emoji>,
    #[serde(default)]
    pub default: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SelectDefaultValue {
    pub id: Snowflake,
    pub r#type: SelectDefaultValueType,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum SelectDefaultValueType {
    User,
    Role,
    Channel,
}

fn one() -> u8 {
    1
}
