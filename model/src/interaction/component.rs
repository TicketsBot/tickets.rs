use super::{ActionRow, Button, InputText, SelectMenu};
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum Component {
    ActionRow(Box<ActionRow>),
    Button(Box<Button>), // Clippy recommendation, large struct
    SelectMenu(Box<SelectMenu>),
    InputText(Box<InputText>),
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Copy, Clone)]
#[repr(u8)]
pub enum ComponentType {
    ActionRow = 1,
    Button = 2,
    SelectMenu = 3,
    InputText = 4,
    UserSelect = 5,
    RoleSelect = 6,
    MentionableSelect = 7,
    ChannelSelect = 8,
}

impl TryFrom<u64> for ComponentType {
    type Error = Box<str>;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Ok(match value {
            1 => Self::ActionRow,
            2 => Self::Button,
            3 => Self::SelectMenu,
            4 => Self::InputText,
            5 => Self::UserSelect,
            6 => Self::RoleSelect,
            7 => Self::MentionableSelect,
            8 => Self::ChannelSelect,
            _ => return Err(format!("invalid component type \"{}\"", value).into_boxed_str()),
        })
    }
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
            ComponentType::SelectMenu
            | ComponentType::UserSelect
            | ComponentType::RoleSelect
            | ComponentType::MentionableSelect
            | ComponentType::ChannelSelect => {
                serde_json::from_value(value).map(Component::SelectMenu)
            }
            ComponentType::InputText => serde_json::from_value(value).map(Component::InputText),
        }
        .map_err(D::Error::custom)?;

        Ok(component)
    }
}
