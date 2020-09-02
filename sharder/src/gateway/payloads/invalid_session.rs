use serde::Deserialize;

use super::Opcode;

#[derive(Deserialize, Debug)]
pub struct InvalidSession {
    #[serde(rename = "op")]
    opcode: Opcode,

    #[serde(rename = "d")]
    pub is_resumable: bool,
}
