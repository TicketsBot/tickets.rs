use serde::{Serialize, Deserialize};
use model::Snowflake;

pub const KEY: &str = "tickets:tokenchange";

#[derive(Serialize, Deserialize, Debug)]
pub struct Payload {
    pub token: String,
    pub new_id: Snowflake,
    pub old_id: Snowflake,
}
