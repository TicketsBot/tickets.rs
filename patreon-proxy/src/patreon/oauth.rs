use std::{sync::Arc, time::Duration};

use crate::{Config, Error, Result};
use tracing::error;

use super::{models::Tokens, ratelimiter::RateLimiter};

pub struct OauthClient<T: RateLimiter> {
    client: reqwest::Client,
    config: Arc<Config>,
    ratelimiter: Arc<T>,
}

const TOKEN_URI: &'static str = "https://www.patreon.com/api/oauth2/token";
const USER_AGENT: &'static str =
    "ticketsbot.net/patreon-proxy (https://github.com/TicketsBot/tickets.rs)";

impl<T: RateLimiter> OauthClient<T> {
    pub fn new(config: Arc<Config>, ratelimiter: Arc<T>) -> Result<Self> {
        let client = reqwest::ClientBuilder::new()
            .use_rustls_tls()
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(15))
            .build()?;

        Ok(OauthClient { client, config, ratelimiter })
    }

    pub async fn grant_credentials(&self) -> Result<Tokens> {
        let form_body = [
            ("grant_type", "client_credentials"),
            ("client_id", self.config.patreon_client_id.as_str()),
            ("client_secret", self.config.patreon_client_secret.as_str()),
        ];

        self.ratelimiter.wait().await;

        let res = self
            .client
            .post(TOKEN_URI)
            .form(&form_body)
            .header("User-Agent", USER_AGENT)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let body = match res.bytes().await {
                Ok(b) => String::from_utf8_lossy(&b).to_string(),
                Err(e) => {
                    error!(error = %e, "Failed to ready response body for non-2xx status code");
                    String::from("Failed to read response body")
                }
            };

            error!(%status, %body, "Failed to perform client_credentials exchange");
            return Error::PatreonError(status).into();
        }

        Ok(res.json().await?)
    }
}
