use serde::{Serialize, Deserialize};

use super::{Activity, ActivityType, ClientStatus, StatusType, User};
use crate::model::Snowflake;
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug)]
pub struct PresenceUpdate {
    pub user: User,
    pub roles: Vec<Snowflake>,
    pub game: Option<Activity>,
    pub guild_id: Snowflake,
    pub status: StatusType,
    pub activities: Vec<Activity>,
    pub client_status: ClientStatus,
    pub premium_since: Option<DateTime<Utc>>,
    pub nick: Option<String>,
}
