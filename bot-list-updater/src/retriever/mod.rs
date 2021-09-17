use crate::UpdaterError;
use serde::{Deserialize, Serialize};

pub struct Retriever {
    http_client: reqwest::Client,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerCounterResponse {
    pub success: bool,
    pub count: usize,
}

impl Retriever {
    pub fn new() -> Retriever {
        Retriever {
            http_client: reqwest::Client::new(),
        }
    }

    pub fn new_with_client(http_client: reqwest::Client) -> Retriever {
        Retriever {
            http_client,
        }
    }

    pub async fn get_count(&self) -> Result<usize, UpdaterError> {
        let url = "https://servercounter.ticketsbot.net/total"; // TODO: Don't hardcode

        let res: ServerCounterResponse = self.http_client.get(url)
            .send()
            .await
            .map_err(UpdaterError::ReqwestError)?
            .json()
            .await
            .map_err(UpdaterError::ReqwestError)?;

        Ok(res.count)
    }
}

impl Default for Retriever {
    fn default() -> Self {
        Self::new()
    }
}