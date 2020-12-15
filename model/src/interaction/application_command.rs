use serde::{Serialize, Deserialize};
use crate::Snowflake;
use crate::interaction::ApplicationCommandOption;

#[derive(Serialize, Deserialize, Debug)]
pub struct ApplicationCommand {
    pub id: Snowflake,
    pub application_id: Snowflake,
    pub name: Box<str>,
    pub description: Box<str>,
    pub options: Vec<ApplicationCommandOption>,
}