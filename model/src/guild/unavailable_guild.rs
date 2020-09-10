use serde::{Serialize, Deserialize};

use crate::Snowflake;

#[derive(Serialize, Deserialize, Debug)]
pub struct UnavailableGuild {
    pub id: Snowflake,
    #[serde(default)]
    pub unavailable: bool,
}