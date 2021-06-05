use super::Opcode;
use model::user::StatusUpdate;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct PresenceUpdate {
    #[serde(rename = "op")]
    opcode: Opcode,

    #[serde(rename = "d")]
    data: StatusUpdate,
}

impl PresenceUpdate {
    pub fn new(presence: StatusUpdate) -> PresenceUpdate {
        PresenceUpdate {
            opcode: Opcode::PresenceUpdate,
            data: presence,
        }
    }
}
