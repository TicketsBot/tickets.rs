pub use serde::Deserialize;

use crate::model::{Snowflake, PermissionBitSet};

#[derive(Deserialize, Debug)]
pub struct PermissionOverwrite {
    pub id: Snowflake,

    #[serde(rename = "type")]
    pub overwrite_type: PermissionOverwriteType,

    pub allow: u32,

    #[serde(skip_serializing)]
    pub allow_new: PermissionBitSet,

    pub deny: u32,

    #[serde(skip_serializing)]
    pub deny_new: PermissionBitSet,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum PermissionOverwriteType {
    Role,
    Member,
}