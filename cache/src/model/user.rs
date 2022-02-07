use model::user::{PremiumType, User};
use model::{Discriminator, ImageHash, Snowflake};

#[derive(Clone, Debug)]
pub struct CachedUser {
    pub username: Box<str>,
    pub discriminator: Discriminator,
    pub avatar: Option<ImageHash>,
    pub bot: bool,
    pub premium_type: Option<PremiumType>,
    pub public_flags: u64,
}

impl CachedUser {
    pub fn into_user(self, id: Snowflake) -> User {
        User {
            id,
            username: self.username.to_string(),
            discriminator: self.discriminator,
            avatar: self.avatar,
            bot: self.bot,
            // oauth fields
            system: false,
            mfa_enabled: None,
            locale: None,
            verified: None,
            email: None,
            flags: 0,
            // end of oauth fields
            premium_type: self.premium_type,
            public_flags: self.public_flags,
        }
    }
}

impl From<User> for CachedUser {
    fn from(other: User) -> Self {
        Self {
            username: other.username.into_boxed_str(),
            discriminator: other.discriminator,
            avatar: other.avatar,
            bot: other.bot,
            premium_type: other.premium_type,
            public_flags: other.public_flags,
        }
    }
}
