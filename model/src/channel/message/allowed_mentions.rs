use serde::{Serialize, Deserialize};
use crate::Snowflake;

#[derive(Serialize, Deserialize, Debug)]
pub struct AllowedMentions {
    pub parse: Option<Vec<Box<str>>>,
    pub roles: Option<Vec<Snowflake>>,
    pub users: Option<Vec<Snowflake>>,
    pub replied_user: Option<bool>,
}