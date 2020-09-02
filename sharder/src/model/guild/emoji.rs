use serde::Deserialize;

use crate::model::{Snowflake, PermissionBitSet};
use crate::model::user::User;

#[derive(Deserialize, Debug)]
pub struct Emoji {
    pub id: Option<Snowflake>,
    pub name: Option<String>,
    pub roles: Option<Vec<Snowflake>>,
    pub user: Option<User>,
    pub requires_colons: Option<bool>,
    pub managed: Option<bool>,
    pub animated: Option<bool>,
    pub available: Option<bool>,
}