use crate::Snowflake;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ThreadMetadata {
    pub archived: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archiver_id: Option<Snowflake>,
    pub auto_archive_duration: ThreadArchiveDuration,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Copy, Clone)]
#[repr(u16)]
pub enum ThreadArchiveDuration {
    Hour = 60,
    Day = 1440,
    ThreeDays = 4320,
    Week = 10080,
}
