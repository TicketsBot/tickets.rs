use serde::Deserialize;

use super::Opcode;

#[derive(Deserialize, Debug)]
pub struct HeartbeatAck {
    #[serde(rename = "op")]
    opcode: Opcode,
}
