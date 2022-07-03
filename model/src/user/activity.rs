use serde::{Deserialize, Serialize};

use super::{ActivityEmoji, ActivityType, Assets, Party, Secrets, Timestamps};
use crate::Snowflake;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Activity {
    pub name: String,

    #[serde(rename = "type")]
    pub activity_type: ActivityType,

    /// only valid when activity_type = streaming
    pub url: Option<String>,

    #[serde(skip_serializing)]
    pub created_at: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamps: Option<Timestamps>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub application_id: Option<Snowflake>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<ActivityEmoji>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub party: Option<Party>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets: Option<Assets>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub secrets: Option<Secrets>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u64>,
}

impl Activity {
    pub fn new(name: String, activity_type: ActivityType) -> Activity {
        Activity {
            name,
            activity_type,
            url: None,
            created_at: 0,
            timestamps: None,
            application_id: None,
            details: None,
            state: None,
            emoji: None,
            party: None,
            assets: None,
            secrets: None,
            instance: None,
            flags: None,
        }
    }
}
