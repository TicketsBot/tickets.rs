use serde::Deserialize;

use super::Opcode;

#[derive(Deserialize, Debug)]
pub struct Reconnect {
    #[serde(rename = "op")]
    opcode: Opcode,
}
