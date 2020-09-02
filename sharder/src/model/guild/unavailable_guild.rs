use serde::Deserialize;

use crate::model::Snowflake;

#[derive(Deserialize, Debug)]
pub struct UnavailableGuild {
    pub id: Snowflake,
    pub unavailable: bool,
}