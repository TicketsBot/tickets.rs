use crate::channel::Permission;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::fmt::Formatter;

#[derive(Debug, Clone, Copy)]
pub struct PermissionBitSet(pub u64);

impl PermissionBitSet {
    pub fn has_permission(&self, permission: Permission) -> bool {
        let perm = permission as u64;
        self.0 & perm == perm
    }
}

impl Serialize for PermissionBitSet {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for PermissionBitSet {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(PermissionBitSet(
            String::deserialize(deserializer)?
                .parse()
                .map_err(Error::custom)?,
        ))
    }
}

impl fmt::Display for PermissionBitSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
