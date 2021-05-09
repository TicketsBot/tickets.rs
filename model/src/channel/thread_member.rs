use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use crate::Snowflake;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ThreadMember {
    pub id: Snowflake,
    pub user_id: Snowflake,
    pub join_timestamp: DateTime<Utc>,
    pub flags: usize,
}