use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Heartbeat {
    #[serde(rename = "d")]
    seq: Option<usize>,
}

impl Heartbeat {
    pub fn new(seq: Option<usize>) -> Heartbeat {
        Heartbeat {
            seq,
        }
    }
}
