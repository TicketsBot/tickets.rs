use serde::Deserialize;

use crate::model::{Snowflake};
use crate::model::user::User;
use chrono::{DateTime, Utc};

#[derive(Deserialize, Debug)]
pub struct Member {
    pub user: Option<User>,
    pub nick: Option<String>,
    pub roles: Vec<Snowflake>,
    pub joined_at: DateTime<Utc>,
    pub premium_since: Option<DateTime<Utc>>,
    pub deaf: bool,
    pub mute: bool,
}