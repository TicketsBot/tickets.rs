use serde::Serialize;

pub const KEY: &str = "tickets:events";

#[derive(Serialize, Debug)]
pub struct Event<'a> {
    pub bot_token: String,
    pub bot_id: u64,
    pub is_whitelabel: bool,
    pub shard_id: u16,
    pub event_type: String,
    pub data: &'a serde_json::Value,
}
