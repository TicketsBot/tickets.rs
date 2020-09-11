use super::Opcode;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Payload {
    #[serde(rename = "op")]
    pub opcode: Opcode,

    #[serde(rename = "s")]
    pub seq: Option<usize>,
}