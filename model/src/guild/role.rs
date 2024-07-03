use serde::{Deserialize, Serialize};

use crate::{PermissionBitSet, Snowflake};

#[derive(Serialize, Deserialize, Debug)]
pub struct Role {
    #[serde(skip_serializing)]
    pub id: Snowflake,
    pub name: String,
    pub color: u32,
    pub hoist: bool,
    pub position: i16,
    pub permissions: PermissionBitSet,
    pub managed: bool,
    pub mentionable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<RoleTags>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RoleTags {
    #[serde(skip_serializing_if = "Option::is_none")]
    bot_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    integration_id: Option<Snowflake>,
}

impl PartialEq for Role {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
