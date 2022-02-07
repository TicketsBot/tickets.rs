use chrono::{DateTime, Utc};
use model::guild::Member;
use model::user::User;
use model::Snowflake;

#[derive(Clone, Debug)]
pub struct CachedMember {
    pub nick: Option<Box<str>>,
    pub roles: Vec<Snowflake>,
    pub joined_at: DateTime<Utc>,
    pub premium_since: Option<DateTime<Utc>>,
    pub deaf: bool,
    pub mute: bool,
    pub pending: bool,
}

impl CachedMember {
    pub fn into_member(self, user: User) -> Member {
        Member {
            user: Some(user),
            nick: self.nick,
            roles: self.roles,
            joined_at: self.joined_at,
            premium_since: self.premium_since,
            deaf: self.deaf,
            mute: self.mute,
            pending: self.pending,
        }
    }
}

impl From<Member> for CachedMember {
    fn from(other: Member) -> Self {
        Self {
            nick: other.nick,
            roles: other.roles,
            joined_at: other.joined_at,
            premium_since: other.premium_since,
            deaf: other.deaf,
            mute: other.mute,
            pending: other.pending,
        }
    }
}
