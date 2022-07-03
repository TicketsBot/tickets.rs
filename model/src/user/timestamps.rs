use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Timestamps {
    pub start: Option<u64>,
    pub end: Option<u64>,
}

impl Timestamps {
    pub fn new(start: Option<u64>, end: Option<u64>) -> Timestamps {
        Timestamps { start, end }
    }
}
