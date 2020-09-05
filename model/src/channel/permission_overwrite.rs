pub use serde::{Serialize, Deserialize};

use crate::{Snowflake, PermissionBitSet};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PermissionOverwrite {
    pub id: Snowflake,

    #[serde(rename = "type")]
    pub overwrite_type: PermissionOverwriteType,

    pub allow: u32,

    #[serde(skip_serializing)]
    pub allow_new: Option<PermissionBitSet>,

    pub deny: u32,

    #[serde(skip_serializing)]
    pub deny_new: Option<PermissionBitSet>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum PermissionOverwriteType {
    Role,
    Member,
}