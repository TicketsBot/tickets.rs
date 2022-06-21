use crate::UpdaterError;
use serde::{Deserialize, Serialize};

pub struct Retriever {
    base_url: String,
    http_client: reqwest::Client,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerCounterResponse {
    pub success: bool,
    pub count: usize,
}

impl Retriever {
    pub fn new(base_url: String) -> Retriever {
        Retriever {
            base_url,
            http_client: reqwest::Client::new(),
        }
    }

    pub fn new_with_client(base_url: String, http_client: reqwest::Client) -> Retriever {
        Retriever {
            base_url,
            http_client,
        }
    }

    pub async fn get_count(&self) -> Result<usize, UpdaterError> {
        let url = format!("{}/total", self.base_url);

        let res: ServerCounterResponse = self
            .http_client
            .get(url)
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
        Self::new("https://servercounter.ticketsbot.net".to_owned())
    }
}
