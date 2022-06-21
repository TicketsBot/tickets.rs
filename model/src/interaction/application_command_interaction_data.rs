use crate::interaction::{
    ApplicationCommandInteractionDataOption, ApplicationCommandInteractionDataResolved,
    ApplicationCommandType, ComponentType,
};
use crate::Snowflake;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ApplicationCommandInteractionData {
    pub id: Snowflake,
    pub name: Box<str>,
    #[serde(default)]
    pub resolved: ApplicationCommandInteractionDataResolved,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<ApplicationCommandInteractionDataOption>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_id: Option<Box<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_type: Option<ComponentType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<Snowflake>,
    pub r#type: ApplicationCommandType,
}
