use super::Opcode;
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct Identify {
    #[serde(rename = "op")]
    opcode: Opcode,

    #[serde(rename = "d")]
    pub data: IdentifyData,
}

impl Identify {
    pub fn new(token: String) -> Identify {
        Identify {
            opcode: Opcode::Identify,
            data: IdentifyData {
                token,
            }
        }
    }
}

#[derive(Serialize, Debug)]
pub struct IdentifyData {
    pub token: String,
}