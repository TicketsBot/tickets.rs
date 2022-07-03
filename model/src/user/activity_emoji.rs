use serde::{Deserialize, Serialize};

use crate::Snowflake;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActivityEmoji {
    pub name: String,
    pub id: Option<Snowflake>,
    pub animated: Option<bool>,
}

impl ActivityEmoji {
    #[allow(dead_code)]
    pub fn new(emoji: String) -> ActivityEmoji {
        ActivityEmoji {
            name: emoji,
            id: None,
            animated: None,
        }
    }

    #[allow(dead_code)]
    pub fn new_custom_emoji(name: String, id: Snowflake, animated: bool) -> ActivityEmoji {
        ActivityEmoji {
            name,
            id: Some(id),
            animated: Some(animated),
        }
    }
}
