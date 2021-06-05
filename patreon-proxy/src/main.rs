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
use std::error::Error;
use std::sync::Arc;

use chrono::prelude::*;

use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::delay_for;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    let config = Arc::new(Config::new().unwrap());

    let db_client = Database::new(&config).await?;
    db_client.create_schema().await?;

    let mut tokens = Arc::new(
        db_client
            .get_tokens(config.patreon_client_id.clone())
            .await?,
    );
    tokens = Arc::new(handle_refresh(&tokens, &config, &db_client).await?);

    let mut poller = Poller::new(Arc::clone(&config), Arc::clone(&tokens));

    let mut server_started = false;

    let data = Arc::new(RwLock::new(HashMap::<String, patreon::Tier>::new()));

    loop {
        if (current_time_seconds() - 86400) > tokens.expires {
            tokens = Arc::new(handle_refresh(&tokens, &config, &db_client).await?);
            poller.tokens = Arc::clone(&tokens);
        }

        match poller.poll().await {
            Ok(patrons) => {
                {
                    let mut map = data.write().await;
                    *map = patrons;
                }

                if !server_started {
                    start_server(Arc::clone(&config), Arc::clone(&data));
                    server_started = true;
                }
            }
            Err(e) => eprintln!("An error occured whilst polling Patreon: {}", e),
        };

        delay_for(Duration::from_secs(30)).await;
    }
}

fn current_time_seconds() -> i64 {
    Utc::now().timestamp()
}

async fn handle_refresh(
    tokens: &database::Tokens,
    config: &Config,
    db_client: &Database,
) -> Result<database::Tokens, Box<dyn Error>> {
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
