use serde::Deserialize;

use crate::model::{Snowflake, PermissionBitSet};

#[derive(Deserialize, Debug)]
pub struct Role {
    pub id: Snowflake,
    pub name: String,
    pub color: u16,
    pub hoist: bool,
    pub position: u8,
    pub permissions: u32,
    pub permissions_new: PermissionBitSet,
    pub managed: bool,
    pub mentionable: bool,
}