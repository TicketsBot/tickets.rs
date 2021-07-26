use crate::event_forwarding::EventForwarder;
use crate::gateway::worker_response::WorkerResponse;
use crate::{Config, GatewayError};
use async_trait::async_trait;
use common::event_forwarding;
use model::Snowflake;
use std::time::Duration;

pub struct HttpEventForwarder {
    client: reqwest::Client,
}

impl HttpEventForwarder {
    pub fn new(client: reqwest::Client) -> HttpEventForwarder {
        HttpEventForwarder {
            client,
        }
    }

    pub fn build_http_client() -> reqwest::Client {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .gzip(cfg!(feature = "compression"))
            .build()
            .expect("build_http_client")
    }
}

#[async_trait]
impl EventForwarder for HttpEventForwarder {
    async fn forward_event(
        &self,
        config: &Config,
        event: event_forwarding::Event<'_>,
        _guild_id: Option<Snowflake>,
    ) -> Result<(), GatewayError> {
        let uri = config.get_worker_svc_uri();

        // reqwest::Client uses Arcs internally, meaning this method clones the same client but
        // allows us to make use of connection pooling
        let req = self.client.clone().post(uri).json(&event);

        let res = req.send().await.map_err(GatewayError::ReqwestError)?;

        let bytes = res.bytes().await.map_err(GatewayError::ReqwestError)?;
        let res: WorkerResponse = serde_json::from_slice(&bytes).map_err(|_| {
            let message = std::str::from_utf8(&*bytes).map_err(GatewayError::Utf8Error);
            match message {
                Ok(message) => GatewayError::WorkerError(Box::from(message)),
                Err(e) => e,
            }
        })?;

        match res.success {
            true => Ok(()),
            false => GatewayError::WorkerError(Box::from(
                res.error
                    .unwrap_or(std::str::from_utf8(&*bytes).map_err(GatewayError::Utf8Error)?),
            ))
            .into(),
        }
    }
}
