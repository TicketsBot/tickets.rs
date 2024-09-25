use crate::event_forwarding::EventForwarder;
use crate::gateway::worker_response::WorkerResponse;
use crate::{Config, GatewayError, Result};
use async_trait::async_trait;
use common::event_forwarding;
use model::Snowflake;
use std::time::Duration;

pub struct HttpEventForwarder {
    client: reqwest::Client,
}

impl HttpEventForwarder {
    pub fn new(client: reqwest::Client) -> HttpEventForwarder {
        HttpEventForwarder { client }
    }

    pub fn build_http_client() -> reqwest::Client {
        let mut builder = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .use_rustls_tls();

        #[cfg(feature = "compression")]
        {
            builder = builder.gzip(true);
        }

        builder.build().expect("build_http_client")
    }
}

#[async_trait]
impl EventForwarder for HttpEventForwarder {
    #[tracing::instrument(skip(self, config, event))]
    async fn forward_event(
        &self,
        config: &Config,
        event: event_forwarding::Event,
        _guild_id: Option<Snowflake>,
    ) -> Result<()> {
        let uri = config.get_worker_svc_uri().expect("worker_svc_uri not set");

        // reqwest::Client uses Arcs internally, meaning this method clones the same client but
        // allows us to make use of connection pooling
        let req = self.client.clone().post(uri).json(&event);

        let res = req.send().await?;
        let bytes = res.bytes().await?;

        let res = serde_json::from_slice::<WorkerResponse>(&bytes);
        let res = match res {
            Ok(v) => v,
            Err(_) => {
                let message = std::str::from_utf8(&bytes)?;
                return GatewayError::WorkerError(message.to_owned()).into();
            }
        };

        if !res.success {
            return Err(GatewayError::WorkerError(
                res.error.unwrap_or(std::str::from_utf8(&bytes)?).to_owned(),
            ));
        }

        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        Ok(())
    }
}

impl Default for HttpEventForwarder {
    fn default() -> Self {
        Self::new(Self::build_http_client())
    }
}
