use serde::Serialize;
use serde_json::value::RawValue;

pub const COMMAND_KEY: &str = "tickets:commands";

#[derive(Serialize, Debug)]
pub struct Command<'a> {
    pub bot_token: &'a str,
    pub bot_id: u64,
    pub is_whitelabel: bool,
    pub data: Box<RawValue>,
}
