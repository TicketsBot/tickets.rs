use serde::Deserialize;

use super::Opcode;

#[derive(Deserialize, Debug)]
pub struct Hello {
    #[serde(rename = "op")]
    opcode: Opcode,

    #[serde(rename = "d")]
    pub data: HelloData,
}

#[derive(Deserialize, Debug)]
pub struct HelloData {
    pub heartbeat_interval: u32,
}
