use std::num::NonZeroU32;
use async_trait::async_trait;

use governor::{clock, middleware::NoOpMiddleware, state::{InMemoryState, NotKeyed}, Quota};

use super::RateLimiter;

pub struct GovernorRateLimiter {
    ratelimiter: governor::RateLimiter<NotKeyed, InMemoryState, clock::DefaultClock, NoOpMiddleware>
}

impl GovernorRateLimiter {
    pub fn new(per_minute: u32) -> Self {
        let ratelimiter = governor::RateLimiter::direct(Quota::per_minute(
            NonZeroU32::new(per_minute).expect("per_minute must be greater than 0"),
        ));
        
        Self {
            ratelimiter,
        }
    }
}

#[async_trait]
impl RateLimiter for GovernorRateLimiter {
    async fn wait(&self) {
        self.ratelimiter.until_ready().await;
    }
}
