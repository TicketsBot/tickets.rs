use serde::{Serialize, Deserialize};
use crate::guild::Emoji;

#[derive(Serialize, Deserialize, Debug)]
pub struct Reaction {
    pub count: usize,
    pub me: bool,
    pub emoji: Emoji,
}