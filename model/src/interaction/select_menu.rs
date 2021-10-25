use crate::guild::Emoji;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct SelectMenu {
    pub custom_id: Box<str>,
    pub options: Vec<SelectOption>,
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

fn one() -> u8 {
    1
}
