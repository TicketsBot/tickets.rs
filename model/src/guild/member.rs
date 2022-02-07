use serde::{Deserialize, Serialize};

use crate::user::User;
use crate::Snowflake;
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Member {
    pub user: Option<User>,
    pub nick: Option<Box<str>>,
    #[serde(serialize_with = "Snowflake::serialize_vec_to_ints")]
    pub roles: Vec<Snowflake>,
    pub joined_at: DateTime<Utc>,
    pub premium_since: Option<DateTime<Utc>>,
    #[serde(default)]
    pub deaf: bool,
    #[serde(default)]
    pub mute: bool,
    #[serde(default)]
    pub pending: bool,
}

