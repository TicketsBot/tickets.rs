use serde::{Serialize, Deserialize};

use super::{Activity, ClientStatus, StatusType, User};
use crate::Snowflake;
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug)]
pub struct PresenceUpdate {
    pub user: User,
    pub roles: Vec<Snowflake>,
    pub game: Option<Activity>,
    pub guild_id: Option<Snowflake>,
    pub status: StatusType,
    pub activities: Vec<Activity>,
    pub client_status: ClientStatus,
    pub premium_since: Option<DateTime<Utc>>,
    pub nick: Option<String>,
}
