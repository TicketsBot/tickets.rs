mod config;
mod database;
mod error;
mod http;
mod patreon;

use config::Config;
use database::Database;
use patreon::oauth;
use patreon::Entitlement;
use patreon::Poller;

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, RwLock};

use chrono::prelude::*;

use sentry::types::Dsn;
use sentry_tracing::EventFilter;
use std::time::Duration;
use tokio::time::sleep;

use crate::error::Error;
use tracing::log::{debug, error, info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

#[tokio::main]
pub async fn main() -> Result<(), Error> {
    let config = Config::new().expect("Failed to load config from environment variables");

    let _guard = configure_observability(&config);

    info!("Connecting to database...");
    let db_client = Database::connect(&config).await?;
    db_client.create_schema().await?;
    info!("Database connection established");

    let mut tokens = Arc::new(
        db_client
            .get_tokens(config.patreon_client_id.clone())
            .await?,
    );
    tokens = Arc::new(handle_refresh(&tokens, &config, &db_client).await?);

    let mut poller = Poller::new(config.patreon_campaign_id.clone(), Arc::clone(&tokens));

    let mut server_started = false;

    let data: Arc<RwLock<HashMap<String, Vec<Entitlement>>>> =
        Arc::new(RwLock::new(HashMap::new()));

    let last_poll_time = Arc::new(RwLock::new(
        DateTime::from_timestamp_millis(0).expect("Failed to create DateTime from timestamp"),
    ));

    loop {
        info!("Starting loop");

        // Patreon issues tokens that last 1 month, but I have noticed some issues refreshing them close to the deadline
        if (current_time_seconds() + (86400 * 3)) > tokens.expires {
            info!("Needs new credentials");
            tokens = Arc::new(handle_refresh(&tokens, &config, &db_client).await?);
            poller.tokens = Arc::clone(&tokens);
            info!("Retrieved new credentials");
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

                if !server_started {
                    start_server(&config, Arc::clone(&data), Arc::clone(&last_poll_time));
                    server_started = true;
                }
            }
            Err(e) => error!("An error occured whilst polling Patreon: {}", e),
        };

        sleep(Duration::from_secs(30)).await;
    }
}

fn current_time_seconds() -> i64 {
    Utc::now().timestamp()
}

async fn handle_refresh(
    tokens: &database::Tokens,
    config: &Config,
    db_client: &Database,
) -> Result<database::Tokens, Error> {
    info!("handle_refresh called");
    let new_tokens = oauth::refresh_tokens(
        tokens.refresh_token.clone(),
        config.patreon_client_id.clone(),
        config.patreon_client_secret.clone(),
    )
    .await?;

    let tokens = database::Tokens::new(
        new_tokens.access_token,
        new_tokens.refresh_token,
        current_time_seconds() + new_tokens.expires_in,
    );
    db_client
        .update_tokens(config.patreon_client_id.clone(), &tokens)
        .await?;

    Ok(tokens)
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
