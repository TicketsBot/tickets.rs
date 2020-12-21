use model::user::StatusUpdate;
use model::Snowflake;

pub struct Options {
    pub token: Box<str>,
    pub shard_count: ShardCount,
    pub presence: StatusUpdate,
    pub large_sharding_buckets: u16,
    pub user_id: Snowflake,
}

pub struct ShardCount {
    pub total: u16,
    pub lowest: u16,
    // Inclusive
    pub highest: u16, // Exclusive
}
