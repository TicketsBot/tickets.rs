use serde::Deserialize;

use super::Opcode;
use crate::gateway::payloads::event::Event;

#[derive(Deserialize, Debug)]
pub struct Dispatch {
    #[serde(rename = "op")]
    opcode: Opcode,

    #[serde(rename = "d", flatten)]
    pub data: Event,

    #[serde(rename = "s")]
    pub seq: usize,
}
