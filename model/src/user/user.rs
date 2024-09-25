use serde::{Deserialize, Serialize};

use super::PremiumType;
use crate::{ImageHash, Snowflake};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    #[serde(skip_serializing)]
    pub id: Snowflake,
    pub username: String,
    pub global_name: Option<String>,
    pub avatar: Option<ImageHash>,
    #[serde(default)]
    pub bot: bool,
    #[serde(default)]
    pub system: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mfa_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub premium_type: Option<PremiumType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_flags: Option<u64>,
}

impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        self.id.0 == other.id.0
    }
}

impl User {
    pub fn blank(user_id: Snowflake) -> User {
        User {
            id: user_id,
            username: "".to_string(),
            global_name: None,
            avatar: None,
            bot: false,
            system: false,
            mfa_enabled: None,
            locale: None,
            verified: None,
            email: None,
            flags: None,
            premium_type: None,
            public_flags: None,
        }
    }
}
