use async_trait::async_trait;
use crate::gateway::Shard;
use std::collections::HashMap;
use tokio::task::JoinHandle;

#[async_trait]
pub trait ShardManager {
    fn connect(shards: HashMap<u16, Shard>) -> Vec<JoinHandle<()>>;
    fn is_whitelabel() -> bool;

    fn get_join_handles(&mut self) -> &mut Vec<JoinHandle<()>>;
}