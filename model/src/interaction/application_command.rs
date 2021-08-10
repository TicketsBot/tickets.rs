use crate::interaction::{ApplicationCommandOption, ApplicationCommandType};
use crate::Snowflake;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ApplicationCommand {
    pub id: Snowflake,
    pub r#type: ApplicationCommandType,
    pub application_id: Snowflake,
    pub name: Box<str>,
    pub description: Box<str>,
    #[serde(default)]
    pub options: Vec<ApplicationCommandOption>,
    #[serde(default = "returns_true")]
    pub default_permission: bool,
}

fn returns_true() -> bool {
    true
}
