use model::user::StatusUpdate;

pub struct Options {
    pub token: String,
    pub shard_count: ShardCount,
    pub presence: StatusUpdate,
    pub large_sharding_buckets: u16,
}

pub struct ShardCount {
    pub total: u16,
    pub lowest: u16, // Inclusive
    pub highest: u16, // Exclusive
}
