use serde::Deserialize;
use serde_json::value::Value;

use super::Opcode;
use super::event::Event;

#[derive(Deserialize, Debug)]
pub struct Dispatch {
    #[serde(rename = "op")]
    opcode: Opcode,

    #[serde(rename = "d", flatten)]
    pub data: Event,
}
