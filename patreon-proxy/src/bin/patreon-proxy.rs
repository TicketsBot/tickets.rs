use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, RwLock};

use chrono::prelude::*;

use patreon_proxy::patreon::oauth::OauthClient;
use patreon_proxy::patreon::ratelimiter::{GovernorRateLimiter, RateLimiter};
use patreon_proxy::patreon::{Entitlement, Poller, Tokens};
use patreon_proxy::{http, Config, Result};
use sentry::types::Dsn;
use sentry_tracing::EventFilter;
use std::time::{Duration, Instant};
use tokio::time::sleep;

use tracing::{debug, error, info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

#[tokio::main]
pub async fn main() -> Result<()> {
    let config = Arc::new(Config::new().expect("Failed to load config from environment variables"));

    let _guard = configure_observability(&config);

    let ratelimiter = Arc::new(GovernorRateLimiter::new(config.requests_per_minute));
    let oauth_client = OauthClient::new(Arc::clone(&config), Arc::clone(&ratelimiter))?;

    let tokens = attempt_grant(&oauth_client, Some(10)).await?;
    let expires_at = Instant::now() + Duration::from_secs(tokens.expires_in as u64);

    let mut poller = Poller::new(config.patreon_campaign_id.clone(), tokens, Arc::clone(&ratelimiter));

    let data: Arc<RwLock<HashMap<String, Vec<Entitlement>>>> =
        Arc::new(RwLock::new(HashMap::new()));

    let last_poll_time = Arc::new(RwLock::new(
        DateTime::from_timestamp_millis(0).expect("Failed to create DateTime from timestamp"),
    ));

    start_server(&config, Arc::clone(&data), Arc::clone(&last_poll_time));

    loop {
        info!("Starting loop");

        // Patreon issues tokens that last 1 month, but I have noticed some issues refreshing them close to the deadline
        if (expires_at - Instant::now()) < Duration::from_secs(86400 * 3) {
            info!("Needs new credentials");
            let tokens = attempt_grant(&oauth_client, None).await?;
            poller = Poller::new(config.patreon_campaign_id.clone(), tokens, Arc::clone(&ratelimiter));
            info!(?expires_at, "Retrieved new credentials");
        }

        info!("Polling");
        match poller.poll().await {
            Ok(patrons) => {
                info!("Poll successful, retrieved {} patrons", patrons.len());

                {
                    debug!("Acquiring lock");
                    let mut map = data.write().unwrap();
                    debug!("Lock acquired");
                    *map = patrons;
                    debug!("Overwritten data");
                }

                {
                    let mut last_poll_time = last_poll_time.write().unwrap();
                    *last_poll_time = Utc::now();
                }

                debug!("Data updated");
            }
            Err(e) => error!("An error occured whilst polling Patreon: {}", e),
        };

        sleep(Duration::from_secs(30)).await;
    }
}

async fn attempt_grant<T: RateLimiter>(oauth_client: &OauthClient<T>, max_retries: Option<usize>) -> Result<Tokens> {
    let mut retries = 0usize;

    loop {
        info!("Attempting to refresh credentials");

        let err = match oauth_client.grant_credentials().await {
            Ok(tokens) => return Ok(tokens),
            Err(e) => {
                error!(error = %e, "Failed to refresh credentials, waiting 30s");
                e
            }
        };

        retries += 1;
        if let Some(max) = max_retries {
            if retries >= max {
                error!("Failed to refresh credentials after {} attempts", max);
                return Err(err);
            }
        }

        sleep(Duration::from_secs(30)).await;
    }
}

fn configure_observability(config: &Config) -> sentry::ClientInitGuard {
    let _guard = sentry::init(sentry::ClientOptions {
        dsn: config
            .sentry_dsn
            .clone()
            .map(|dsn| Dsn::from_str(dsn.as_str()).expect("Invalid DSN")),
        debug: config.debug_mode,
        release: sentry::release_name!(),
        ..Default::default()
    });

    let sentry_layer = sentry_tracing::layer().event_filter(|meta| match meta.level() {
        &tracing::Level::ERROR | &tracing::Level::WARN => EventFilter::Exception,
        _ => EventFilter::Ignore,
    });

    let registry = tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(sentry_layer);

    if config.json_log {
        registry
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        registry.with(tracing_subscriber::fmt::layer()).init();
    }

    _guard
}

fn start_server(
    config: &Config,
    patrons: Arc<RwLock<HashMap<String, Vec<Entitlement>>>>,
    last_poll_time: Arc<RwLock<DateTime<Utc>>>,
) {
    let server_addr = config.server_addr.clone();

    tokio::spawn(async move {
        info!("Starting server...");
        http::listen(server_addr.as_str(), patrons, last_poll_time).await;
    });
}
