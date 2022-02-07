use model::guild::Role;
use model::{PermissionBitSet, Snowflake};

#[derive(Clone, Debug)]
pub struct CachedRole {
    pub name: Box<str>,
    pub color: u32,
    pub hoist: bool,
    pub position: i16,
    pub permissions: PermissionBitSet,
    pub managed: bool,
    pub mentionable: bool,
}

impl CachedRole {
    pub fn into_role(self, id: Snowflake) -> Role {
        Role {
            id,
            name: self.name.to_string(),
            color: self.color,
            hoist: self.hoist,
            position: self.position,
            permissions: self.permissions,
            managed: self.managed,
            mentionable: self.mentionable,
        }
    }
}

impl From<Role> for CachedRole {
    fn from(other: Role) -> Self {
        Self {
            name: other.name.into_boxed_str(),
            color: other.color,
            hoist: other.hoist,
            position: other.position,
            permissions: other.permissions,
            managed: other.managed,
            mentionable: other.mentionable,
        }
    }
}
