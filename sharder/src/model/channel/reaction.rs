use serde::Deserialize;
use crate::model::guild::Emoji;

#[derive(Deserialize, Debug)]
pub struct Reaction {
    pub count: usize,
    pub me: bool,
    pub emoji: Emoji,
}