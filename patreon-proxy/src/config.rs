use std::env;

#[derive(Debug)]
pub struct Config {
    pub patreon_campaign_id: String,
    pub patreon_client_id: String,
    pub patreon_client_secret: String,
    pub patreon_redirect_uri: String,
    pub server_addr: String,
    pub server_key: String,
    pub database_uri: String,
}

impl Config {
    pub fn new() -> Result<Config, env::VarError> {
        Ok(Config {
            patreon_campaign_id: env::var("PATREON_CAMPAIGN_ID")?,
            patreon_client_id: env::var("PATREON_CLIENT_ID")?,
            patreon_client_secret: env::var("PATREON_CLIENT_SECRET")?,
            patreon_redirect_uri: env::var("PATREON_REDIRECT_URI")?,
            server_addr: env::var("SERVER_ADDR")?,
            server_key: env::var("SERVER_KEY")?,
            database_uri: env::var("DATABASE_URI")?,
        })
    }
}
