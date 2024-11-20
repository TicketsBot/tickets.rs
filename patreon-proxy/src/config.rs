use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub patreon_campaign_id: String,
    pub patreon_client_id: String,
    pub patreon_client_secret: String,
    pub server_addr: String,
    pub sentry_dsn: Option<String>,
    #[serde(default)]
    pub debug_mode: bool,
    #[serde(default = "returns_true")]
    pub json_log: bool,
    pub requests_per_minute: u32,
}

impl Config {
    pub fn new() -> Result<Config, envy::Error> {
        envy::from_env()
    }
}

fn returns_true() -> bool {
    true
}
