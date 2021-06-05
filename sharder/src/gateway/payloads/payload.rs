use super::Opcode;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Payload {
    #[serde(rename = "op")]
    pub opcode: Opcode,

    #[serde(rename = "s")]
    pub seq: Option<usize>,
}
