use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::Snowflake;

#[derive(Serialize, Deserialize, Debug)]
pub struct GuildApplicationCommandPermissions {
    pub id: Snowflake,
    pub application_id: Snowflake,
    pub guild_id: Snowflake,
    pub permissions: Vec<ApplicationCommandPermissions>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApplicationCommandPermissions {
    pub id: Snowflake,
    pub r#type: ApplicationCommandPermissionType,
    pub permission: bool,
}

#[derive(Serialize_repr, Deserialize_repr, Debug)]
#[repr(u8)]
pub enum ApplicationCommandPermissionType {
    Role = 1,
    User = 2,
    Channel = 3,
}
