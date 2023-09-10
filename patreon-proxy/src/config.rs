use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub patreon_campaign_id: String,
    pub patreon_client_id: String,
    pub patreon_client_secret: String,
    pub patreon_redirect_uri: String,
    pub server_addr: String,
    pub database_uri: String,
    pub sentry_dsn: Option<String>,
    #[serde(default)]
    pub debug_mode: bool,
    #[serde(default = "returns_true")]
    pub json_log: bool,
}

impl Config {
    pub fn new() -> Result<Config, envy::Error> {
        envy::from_env()
    }
}

fn returns_true() -> bool {
    true
}
