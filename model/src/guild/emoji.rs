use serde::{Deserialize, Serialize};

use crate::user::User;
use crate::Snowflake;

#[derive(Serialize, Deserialize, Debug)]
pub struct Emoji {
    #[serde(skip_serializing)]
    pub id: Option<Snowflake>,
    pub name: Option<String>,
    pub roles: Option<Vec<Snowflake>>,
    pub user: Option<User>,
    pub requires_colons: Option<bool>,
    pub managed: Option<bool>,
    pub animated: Option<bool>,
    pub available: Option<bool>,
}

// very dodgy but works for our use case
impl PartialEq for Emoji {
    fn eq(&self, other: &Self) -> bool {
        if let (Some(self_id), Some(other_id)) = (self.id, other.id) {
            return self_id == other_id;
        }

        false
    }
}
