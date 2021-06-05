use crate::guild::Emoji;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Reaction {
    pub count: usize,
    pub me: bool,
    pub emoji: Emoji,
}
