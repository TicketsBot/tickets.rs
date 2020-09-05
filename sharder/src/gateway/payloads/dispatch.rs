use serde::Deserialize;

use super::Opcode;

#[derive(Deserialize, Debug)]
pub struct Dispatch {
    #[serde(rename = "op")]
    opcode: Opcode,

    #[serde(rename = "d", flatten)]
    pub data: serde_json::Value,
}
