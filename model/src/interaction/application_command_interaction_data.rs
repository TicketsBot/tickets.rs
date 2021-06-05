use crate::interaction::ApplicationCommandInteractionDataOption;
use crate::Snowflake;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ApplicationCommandInteractionData {
    pub id: Snowflake,
    pub name: Box<str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<ApplicationCommandInteractionDataOption>>,
}
