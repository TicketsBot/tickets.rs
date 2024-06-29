use dashmap::DashSet;
use std::sync::Arc;
use tokio::time::Instant;
use uuid::Uuid;

pub struct UsedTokenStore {
    tokens: Arc<DashSet<Uuid>>,
}

impl UsedTokenStore {
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(DashSet::new()),
        }
    }

    pub fn contains(&self, token: &Uuid) -> bool {
        self.tokens.contains(token)
    }

    pub fn add(&self, token: Uuid, expires_at: Instant) -> bool {
        let is_new = self.tokens.insert(token);

        let tokens = Arc::clone(&self.tokens);
        tokio::spawn(async move {
            tokio::time::sleep_until(expires_at).await;
            tokens.remove(&token);
        });

        is_new
    }
}
