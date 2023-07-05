use async_trait::async_trait;

use std::sync::Arc;

#[async_trait]
pub trait ShardManager {
    async fn connect(self: Arc<Self>);
    async fn shutdown(self: Arc<Self>);
}
