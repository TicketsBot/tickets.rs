use serde::{Serialize, Deserialize};
use serde_repr::{Serialize_repr, Deserialize_repr};

use crate::{Snowflake, PermissionBitSet};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PermissionOverwrite {
    pub id: Snowflake,

    #[serde(rename = "type")]
    pub overwrite_type: PermissionOverwriteType,

    pub allow: PermissionBitSet,

    pub deny: PermissionBitSet,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy)]
#[repr(u8)]
pub enum PermissionOverwriteType {
    Role = 0,
    Member = 1,
}