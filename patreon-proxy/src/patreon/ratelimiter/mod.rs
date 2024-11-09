mod governor_ratelimiter;
pub use governor_ratelimiter::GovernorRateLimiter;

use async_trait::async_trait;

#[async_trait]
pub trait RateLimiter {
    async fn wait(&self);
}
