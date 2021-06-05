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
}

impl PartialEq for Role {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
