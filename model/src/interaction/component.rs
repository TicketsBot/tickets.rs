use crate::guild::Emoji;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::convert::TryFrom;

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum Component {
    ActionRow(ActionRow),
    Button(Button),
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Copy, Clone)]
#[repr(u8)]
pub enum ComponentType {
    ActionRow = 1,
    Button = 2,
}

impl TryFrom<u64> for ComponentType {
    type Error = Box<str>;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Ok(match value {
            1 => Self::ActionRow,
            2 => Self::Button,
            _ => Err(format!("invalid component type \"{}\"", value).into_boxed_str())?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ActionRow {
    pub r#type: ComponentType,
    pub components: Vec<Component>,
}

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
}

impl<'de> Deserialize<'de> for Component {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;

        let component_type = value
            .get("type")
            .and_then(Value::as_u64)
            .ok_or_else(|| Box::from("component type was not an integer"))
            .and_then(ComponentType::try_from)
            .map_err(D::Error::custom)?;

        let component = match component_type {
            ComponentType::ActionRow => serde_json::from_value(value).map(Component::ActionRow),
            ComponentType::Button => serde_json::from_value(value).map(Component::Button),
        }
        .map_err(D::Error::custom)?;

        Ok(component)
    }
}
