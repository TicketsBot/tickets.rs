use crate::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[async_trait]
pub trait SessionStore: Send + Sync + 'static {
    async fn get(&self, shard_id: u64) -> Result<Option<SessionData>>;
    async fn get_bulk(&self, shard_ids: &[u64]) -> Result<HashMap<u64, SessionData>>;
    async fn set(&self, shard_id: u64, info: SessionData) -> Result<()>;
    async fn set_bulk(&self, data: HashMap<u64, SessionData>) -> Result<()>;
    async fn invalidate(&self, shard_id: u64) -> Result<()>;
    async fn invalidate_bulk(&self, shard_ids: &[u64]) -> Result<()>;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SessionData {
    pub seq: usize,
    pub session_id: String,
    pub resume_url: Option<String>,
}

mod redis_store;
pub use redis_store::RedisSessionStore;
