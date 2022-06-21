use super::Updater;
use crate::UpdaterError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub struct DiscordBoatsUpdater {
    token: String,
    bot_id: u64,
    http_client: reqwest::Client,
}

impl DiscordBoatsUpdater {
    pub fn new(token: String, bot_id: u64) -> DiscordBoatsUpdater {
        DiscordBoatsUpdater {
            token,
            bot_id,
            http_client: reqwest::Client::new(),
        }
    }

    pub fn new_with_client(
        token: String,
        bot_id: u64,
        http_client: reqwest::Client,
    ) -> DiscordBoatsUpdater {
        DiscordBoatsUpdater {
            token,
            bot_id,
            http_client,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DiscordBoatsRequest {
    pub server_count: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DiscordBoatsResponse {
    pub error: bool,
    pub message: String,
}

#[async_trait]
impl Updater for DiscordBoatsUpdater {
    async fn update(&self, count: usize) -> Result<(), UpdaterError> {
        let url = format!("https://discord.boats/api/bot/{}", self.bot_id);

        let body = DiscordBoatsRequest {
            server_count: count,
        };

        let res = self
            .http_client
            .post(url)
            .header("Authorization", &self.token[..])
            .json(&body)
            .send()
            .await
            .map_err(UpdaterError::ReqwestError)?;

        if res.status().is_success() {
            Ok(())
        } else {
            let body: DiscordBoatsResponse =
                res.json().await.map_err(UpdaterError::ReqwestError)?;
            UpdaterError::ResponseError(body.message).into()
        }
    }
}
