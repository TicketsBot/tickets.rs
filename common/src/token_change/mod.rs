use model::Snowflake;
use serde::{Deserialize, Serialize};

pub const KEY: &str = "tickets:tokenchange";

#[derive(Serialize, Deserialize, Debug)]
pub struct Payload {
    pub token: String,
    pub new_id: Snowflake,
}
