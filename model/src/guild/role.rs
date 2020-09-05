use serde::{Serialize, Deserialize};

use crate::{Snowflake, PermissionBitSet};

#[derive(Serialize, Deserialize, Debug)]
pub struct Role {
    pub id: Snowflake,
    pub name: String,
    pub color: u32,
    pub hoist: bool,
    pub position: u16,
    pub permissions: u32,
    pub permissions_new: PermissionBitSet,
    pub managed: bool,
    pub mentionable: bool,
}