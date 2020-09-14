use serde::{Serialize, Deserialize};

use crate::Snowflake;
use crate::user::User;
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug)]
pub struct Member {
    #[serde(skip_serializing)]
    pub user: Option<User>,
    pub nick: Option<String>,
    #[serde(serialize_with = "Snowflake::serialize_vec_to_ints")]
    pub roles: Vec<Snowflake>,
    pub joined_at: DateTime<Utc>,
    pub premium_since: Option<DateTime<Utc>>,
    pub deaf: bool,
    pub mute: bool,
}
