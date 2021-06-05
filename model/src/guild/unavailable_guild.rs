use serde::{Deserialize, Serialize};

use crate::Snowflake;

#[derive(Serialize, Deserialize, Debug)]
pub struct UnavailableGuild {
    pub id: Snowflake,
    pub unavailable: Option<bool>,
}
