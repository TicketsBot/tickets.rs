use super::Opcode;
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct Resume {
    #[serde(rename = "op")]
    opcode: Opcode,

    #[serde(rename = "d")]
    data: ResumeData,
}

impl Resume {
    pub fn new(token: String, session_id: String, seq: usize) -> Resume {
        Resume {
            opcode: Opcode::Resume,
            data: ResumeData {
                token,
                session_id,
                seq,
            },
        }
    }
}

#[derive(Serialize, Debug)]
struct ResumeData {
    token: String,
    session_id: String,
    seq: usize,
}
