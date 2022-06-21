use super::Updater;
use crate::UpdaterError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub struct DblUpdater {
    token: String,
    bot_id: u64,
    http_client: reqwest::Client,
}

impl DblUpdater {
    pub fn new(token: String, bot_id: u64) -> DblUpdater {
        DblUpdater {
            token,
            bot_id,
            http_client: reqwest::Client::new(),
        }
    }

    pub fn new_with_client(token: String, bot_id: u64, http_client: reqwest::Client) -> DblUpdater {
        DblUpdater {
            token,
            bot_id,
            http_client,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DblRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_connections: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub users: Option<usize>,
    pub guilds: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shard_id: Option<u16>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DblResponse {
    pub message: String,
}

#[async_trait]
impl Updater for DblUpdater {
    async fn update(&self, count: usize) -> Result<(), UpdaterError> {
        let url = format!(
            "https://discordbotlist.com/api/v1/bots/{}/stats",
            self.bot_id
        );

        let body = DblRequest {
            voice_connections: None,
            users: None,
            guilds: count,
            shard_id: None,
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
            let body: DblResponse = res.json().await.map_err(UpdaterError::ReqwestError)?;
            UpdaterError::ResponseError(body.message).into()
        }
    }
}
