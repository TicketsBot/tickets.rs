use crate::config::Config;
use crate::error::Result;
use crate::AppError;
use reqwest::blocking as reqwest;
use serde::{Deserialize, Serialize};
use std::str;

pub struct Client {
    config: Config,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct Response {
    success: bool,
    count: usize,
}

#[derive(Debug, Serialize)]
struct ChannelUpdateBody {
    name: String,
}

impl Client {
    pub fn new(config: Config) -> Self {
        let client = reqwest::ClientBuilder::new()
            .use_rustls_tls()
            .build()
            .expect("failed to build http client");

        Self { config, client }
    }

    pub fn fetch_server_count(&self) -> Result<usize> {
        let res: Response = self
            .client
            .get(self.config.server_counter_url.clone())
            .send()?
            .json()?;

        Ok(res.count)
    }

    pub fn update_channel(&self, server_count: usize) -> Result<()> {
        let body = ChannelUpdateBody {
            name: format!("Server Count: {}", server_count),
        };

        let url = format!(
            "https://discord.com/api/v10/channels/{}",
            self.config.channel_id
        );
        let res = self
            .client
            .patch(url)
            .header(
                "Authorization",
                format!("Bot {}", self.config.discord_token),
            )
            .json(&body)
            .send()?;

        if res.status().is_success() {
            Ok(())
        } else {
            let bytes = res.bytes()?;
            let res_body = str::from_utf8(bytes.as_ref())?;
            AppError::ResponseError(res_body.to_string()).into()
        }
    }
}
