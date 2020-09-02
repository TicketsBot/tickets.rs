use serde::{Serialize, Deserialize};

use crate::model::{Snowflake, Discriminator, ImageHash};
use super::PremiumType;

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub id: Snowflake,
    pub username: String,
    pub discriminator: Discriminator,
    pub avatar: Option<ImageHash>,
    #[serde(default = "returns_false")]
    pub bot: bool,
    #[serde(default = "returns_false")]
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

fn returns_false() -> bool {
    false
}