use serde::{Serialize, Deserialize};
use model::Snowflake;

pub const KEY: &str = "tickets:events";

#[derive(Serialize, Deserialize, Debug)]
pub struct Event {
    pub bot_token: String,
    pub bot_id: Snowflake,
    pub is_whitelabel: bool,
    pub shard_id: u16,
    pub event_type: String,
    pub data: serde_json::Value,
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Extra {
    pub is_join: bool,
}