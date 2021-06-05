use serde::{Deserialize, Serialize};

use super::{Activity, ClientStatus, StatusType, User};
use crate::Snowflake;

#[derive(Serialize, Deserialize, Debug)]
pub struct PresenceUpdate {
    pub user: User,
    pub guild_id: Option<Snowflake>,
    pub status: StatusType,
    pub activities: Vec<Activity>,
    pub client_status: ClientStatus,
}
