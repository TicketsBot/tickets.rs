use crate::Snowflake;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct AllowedMentions {
    pub parse: Option<Vec<Box<str>>>,
    pub roles: Option<Vec<Snowflake>>,
    pub users: Option<Vec<Snowflake>>,
    pub replied_user: Option<bool>,
}
