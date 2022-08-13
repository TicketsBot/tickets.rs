mod config;
mod database;
mod error;
mod http;
mod patreon;

use config::Config;
use database::Database;
use patreon::oauth;
use patreon::Poller;

use std::collections::HashMap;
use std::sync::Arc;

use chrono::prelude::*;

use parking_lot::RwLock;
use std::time::Duration;
use tokio::time::sleep;

use crate::error::PatreonError;
use log::{debug, error, info};

#[tokio::main]
pub async fn main() -> Result<(), PatreonError> {
    env_logger::init();

    let config = Arc::new(Config::new().unwrap());

    info!("Connecting to database...");
    let db_client = Database::new(&config).await?;
    db_client.create_schema().await?;
    info!("Database connection established");

    let mut tokens = Arc::new(
        db_client
            .get_tokens(config.patreon_client_id.clone())
            .await?,
    );
    tokens = Arc::new(handle_refresh(&tokens, &config, &db_client).await?);

    let mut poller = Poller::new(Arc::clone(&config), Arc::clone(&tokens));

    let mut server_started = false;

    let data: Arc<RwLock<HashMap<String, patreon::Tier>>> = Arc::new(RwLock::new(HashMap::new()));

    loop {
        info!("Starting loop");
        if (current_time_seconds() + 86400) > tokens.expires {
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
                    let mut map = data.write();
                    debug!("Lock acquired");
                    *map = patrons;
                    debug!("Overridden data");
                }

                debug!("Data updated");

                if !server_started {
                    start_server(Arc::clone(&config), Arc::clone(&data));
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
) -> Result<database::Tokens, PatreonError> {
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

fn start_server(config: Arc<Config>, patrons: Arc<RwLock<HashMap<String, patreon::Tier>>>) {
    tokio::spawn(async move {
        println!("Starting server...");
        http::listen(config, patrons).await;
    });
}
