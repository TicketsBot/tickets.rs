use crate::CacheError;
use model::guild::Emoji;
use model::Snowflake;
use std::convert::TryFrom;
use crate::model::CachedUser;

#[derive(Clone, Debug)]
pub struct CachedEmoji {
    pub name: Box<str>,
    pub roles: Option<Vec<Snowflake>>,
    pub user_id: Snowflake,
    pub user: CachedUser,
    pub requires_colons: bool,
    pub managed: bool,
    pub animated: bool,
    pub available: bool,
}

impl CachedEmoji {
    pub fn into_emoji(self, id: Snowflake) -> Emoji {
        Emoji {
            id: Some(id),
            name: Some(self.name),
            roles: self.roles,
            user: Some(self.user.into_user(self.user_id)),
            requires_colons: Some(self.requires_colons),
            managed: Some(self.managed),
            animated: Some(self.animated),
            available: Some(self.available),
        }
    }
}

impl TryFrom<Emoji> for CachedEmoji {
    type Error = CacheError;

    fn try_from(other: Emoji) -> Result<Self, Self::Error> {
        Ok(Self {
            name: other.name.ok_or_else(|| missing_field("name"))?,
            roles: other.roles,
            user_id: other.user.as_ref().ok_or_else(|| missing_field("user"))?.id,
            user: CachedUser::from(other.user.ok_or_else(|| missing_field("user"))?),
            requires_colons: other.requires_colons.unwrap_or(true),
            managed: other.managed.unwrap_or(false),
            animated: other.animated.unwrap_or(false),
            available: other.available.unwrap_or(true),
        })
    }
}

fn missing_field(field: &str) -> CacheError {
    CacheError::MissingField("CachedEmoji".to_string(), field.to_string())
}
