use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{user::User, Snowflake};

#[derive(Serialize, Deserialize, Debug)]
pub struct JoinRequest {
    pub user_id: Snowflake,
    pub user: User,
    pub rejection_reason: Option<String>,
    pub last_seen: Option<DateTime<Utc>>,
    pub join_request_id: Snowflake,
    pub interview_channel_id: Option<Snowflake>,
    pub id: Snowflake,
    pub guild_id: Snowflake,
    pub form_responses: Vec<FormResponse>,
    pub created_at: DateTime<Utc>,
    pub application_status: String,
    pub actioned_by_user: Option<User>,
    pub actioned_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FormResponse {
    pub values: Vec<String>,
    pub response: bool,
    pub required: bool,
    pub label: String,
    pub field_type: String,
    // TODO: Document "description" and "automations"
}
