use async_trait::async_trait;
use crate::{Shard, GatewayError, Config};
use common::event_forwarding;
use std::sync::Arc;
use std::time::Duration;
use crate::gateway::worker_response::WorkerResponse;
use crate::gateway::payloads::event::Event;
use model::Snowflake;
use tokio::time::delay_for;
use tokio::sync::RwLock;
use crate::event_forwarding::EventForwarder;

pub struct HttpEventForwarder {
    client: reqwest::Client,
    cookie: RwLock<Option<Box<str>>>,
}

impl HttpEventForwarder {
    pub fn new(client: reqwest::Client) -> HttpEventForwarder {
        HttpEventForwarder {
            client,
            cookie: RwLock::new(Option::None),
        }
    }

    pub fn build_http_client() -> reqwest::Client {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .gzip(cfg!(feature = "compression"))
            .build()
            .expect("build_http_client")
    }

    pub fn start_reset_cookie_loop(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                delay_for(Duration::from_secs(180)).await;
                *self.cookie.write().await = None;
            }
        });
    }
}

#[async_trait]
impl EventForwarder for HttpEventForwarder {
    async fn forward_event(&self, config: &Config, event: event_forwarding::Event<'_>, guild_id: Option<Snowflake>) -> Result<(), GatewayError> {
        let uri = &*config.worker_svc_uri;

        // reqwest::Client uses Arcs internally, meaning this method clones the same client but
        // allows us to make use of connection pooling
        let mut req = self.client.clone()
            .post(uri)
            .json(&event);

        if let Some(guild_id) = guild_id {
            let header_name = &*config.sticky_cookie;
            req = req.header(header_name, guild_id.0);
        }

        let cookie = self.cookie.read().await;
        if let Some(cookie) = &*cookie {
            let value = format!("{}={}", config.sticky_cookie, cookie);
            req = req.header(reqwest::header::COOKIE, value);
        }
        drop(cookie); // drop here so we can write later

        let res = req.send()
            .await
            .map_err(GatewayError::ReqwestError)?;

        if let Some(cookie) = res.cookies().find(|c| c.name() == &*config.sticky_cookie) {
            // TODO: LOG
            //shard.log(format!("Got new session cookie: {}", cookie.value()));
            *self.cookie.write().await = Some(Box::from(cookie.value()));
        }

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
            false => GatewayError::WorkerError(Box::from(res.error.unwrap_or(std::str::from_utf8(&*bytes).map_err(GatewayError::Utf8Error)?))).into()
        }
    }
}