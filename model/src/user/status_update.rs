use serde::{Deserialize, Serialize};

use super::{Activity, ActivityType, StatusType};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StatusUpdate {
    pub since: Option<u64>,
    pub game: Option<Activity>,
    pub status: StatusType,
    pub afk: bool,
}

impl StatusUpdate {
    pub fn new(
        activity_type: ActivityType,
        status: String,
        status_type: StatusType,
    ) -> StatusUpdate {
        StatusUpdate {
            since: Some(0),
            game: Some(Activity::new(status, activity_type)),
            status: status_type,
            afk: false,
        }
    }
}
