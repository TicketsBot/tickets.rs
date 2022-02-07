use crate::{CacheError, Result, Options};
use model::Snowflake;
use model::guild::Guild;
use reqwest::StatusCode;
use serde::Deserialize;

#[derive(Clone)]
pub struct CacheClient {
    base_url: String,
    client: reqwest::Client,
}

#[derive(Deserialize, Debug)]
struct ErrorResponse {
    error: String,
}

impl CacheClient {
    pub async fn fetch_guild(&self, guild_id: Snowflake, opts: Options) -> Result<Option<Guild>> {
        let url = format!("{}/guilds/{}", self.base_url, guild_id);
        let res=  self.client.get(url)
            .send()
            .await?;

        match res.status() {
            StatusCode::OK => {
                let guild = res.json().await?;
                Ok(guild)
            },
            StatusCode::NOT_FOUND => Ok(None),
            StatusCode::INTERNAL_SERVER_ERROR => {
                let res: ErrorResponse = res.json().await?;
                CacheError::ResponseError(res.error).into()
            }
        }
    }
}
