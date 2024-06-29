use super::ComponentType;
use crate::{guild::Emoji, Snowflake};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Serialize, Deserialize, Debug)]
pub struct Button {
    pub r#type: ComponentType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<Box<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_id: Option<Box<str>>,
    pub style: ButtonStyle,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<Emoji>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sku_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<Box<str>>,
    #[serde(default = "bool::default")]
    pub disabled: bool,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Copy, Clone)]
#[repr(u8)]
pub enum ButtonStyle {
    Primary = 1,
    Secondary = 2,
    Success = 3,
    Danger = 4,
    Link = 5,
    Premium = 6,
}
