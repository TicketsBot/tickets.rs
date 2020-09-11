use serde::{Serialize, Deserialize};

use crate::Snowflake;

#[derive(Serialize, Deserialize, Debug)]
pub struct UnavailableGuild {
    pub id: Snowflake,
    pub unavailable: Option<bool>,
}