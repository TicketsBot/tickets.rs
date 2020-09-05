use serde::Serialize;
use model::Snowflake;

pub const KEY: &str = "tickets:events";

#[derive(Serialize, Debug)]
pub struct Event<'a> {
    pub bot_token: String,
    pub bot_id: Snowflake,
    pub is_whitelabel: bool,
    pub shard_id: u16,
    pub event_type: String,
    pub data: &'a serde_json::Value,
    pub extra: Extra,
}

#[derive(Serialize, Debug)]
pub struct Extra {
    pub is_join: bool,
}