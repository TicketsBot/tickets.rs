#[cfg(feature = "use-sentry")]
use std::default::Default;
use std::str;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

#[cfg(feature = "whitelabel")]
use database::Database;
use deadpool_redis::redis;
use deadpool_redis::{redis::cmd, Pool};
#[cfg(feature = "compression")]
use flate2::{Decompress, FlushDecompress, Status};
use futures::future;
use futures::StreamExt;
use futures_util::SinkExt;
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::value::RawValue;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex as TokioMutex};
use tokio::time::sleep;
use tracing::trace;
use url::Url;

use common::event_forwarding;
use model::Snowflake;

use crate::config::Config;
use crate::gateway::whitelabel_utils::is_whitelabel;
use crate::gateway::{GatewayError, Result};
#[cfg(feature = "whitelabel")]
use crate::payloads::PresenceUpdate;
use crate::InternalCommand;
use crate::ShardIdentifier;

use super::payloads;
use super::payloads::event::Event;
use super::payloads::parser::find_opcode;
use super::payloads::parser::find_seq;
use super::payloads::{Dispatch, Opcode};
use super::session_store::SessionData;
use super::timer;
use super::OutboundMessage;
use crate::gateway::event_forwarding::{is_whitelisted, EventForwarder};
use crate::CloseEvent;
use futures_util::stream::{SplitSink, SplitStream};
use redis::AsyncCommands;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{protocol::frame::coding::CloseCode, Message},
    MaybeTlsStream, WebSocketStream,
};

use tracing::{debug, error, info, warn};

#[cfg(feature = "metrics")]
use lazy_static::lazy_static;

#[cfg(feature = "metrics")]
use prometheus::{register_int_gauge, IntGauge};

#[cfg(feature = "metrics")]
lazy_static! {
    static ref CACHE_BUFFER_GUAGE: IntGauge = register_int_gauge!(
        "cache_buffer",
        "The number of payload that are currently waiting to be stored in the cache"
    )
    .expect("Failed to create cache buffer gauge");
}

const GATEWAY_VERSION: u8 = 10;

pub struct Shard<T: EventForwarder> {
    pub(crate) config: Arc<Config>,
    pub(crate) identify: payloads::Identify,
    large_sharding_buckets: u16,
    redis: Arc<Pool>,
    pub(crate) user_id: Snowflake,
    pub(crate) session_data: Option<SessionData>,
    writer: mpsc::Sender<OutboundMessage>,
    writer_rx: Option<mpsc::Receiver<OutboundMessage>>,
    heartbeat_tx: mpsc::Sender<()>,
    heartbeat_rx: Option<mpsc::Receiver<()>>,
    heartbeat_interval: Duration,
    pub kill_shard_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    kill_shard_rx: Arc<TokioMutex<oneshot::Receiver<()>>>,
    last_ack: Instant,
    last_heartbeat: Instant,
    connect_time: Instant,
    ready_tx: Mutex<Option<oneshot::Sender<()>>>,
    ready_guild_count: u16,
    received_count: usize,
    is_ready: bool,
    #[cfg(feature = "resume-after-identify")]
    used_resume: bool,
    shutdown_rx: broadcast::Receiver<mpsc::Sender<(ShardIdentifier, Option<SessionData>)>>,
    #[cfg(feature = "whitelabel")]
    command_rx: Arc<TokioMutex<mpsc::Receiver<InternalCommand>>>,
    #[cfg(feature = "whitelabel")]
    pub(crate) database: Arc<Database>,
    pub(crate) event_forwarder: Arc<T>,
}

#[cfg(feature = "compression")]
const CHUNK_SIZE: usize = 16 * 1024; // 16KiB

static DEFAULT_GATEWAY_URL: &str = "wss://gateway.discord.gg";

impl<T: EventForwarder> Shard<T> {
    pub fn new(
        config: Arc<Config>,
        identify: payloads::Identify,
        large_sharding_buckets: u16,
        redis: Arc<Pool>,
        user_id: Snowflake,
        event_forwarder: Arc<T>,
        ready_tx: Option<oneshot::Sender<()>>,
        shutdown_rx: broadcast::Receiver<mpsc::Sender<(ShardIdentifier, Option<SessionData>)>>,
        #[cfg(feature = "whitelabel")] database: Arc<Database>,
        #[cfg(feature = "whitelabel")] command_rx: mpsc::Receiver<InternalCommand>,
    ) -> Shard<T> {
        let (kill_shard_tx, kill_shard_rx) = oneshot::channel();
        let (writer_tx, writer_rx) = mpsc::channel(4);
        let (heartbeat_tx, heartbeat_rx) = mpsc::channel(1);

        Shard {
            config,
            identify,
            large_sharding_buckets,
            redis,
            user_id,
            session_data: None,
            writer: writer_tx,
            writer_rx: Some(writer_rx),
            heartbeat_tx,
            heartbeat_rx: Some(heartbeat_rx),
            heartbeat_interval: Duration::from_millis(42500), // This will be overwritten later by the HELLO payload
            kill_shard_tx: Arc::new(Mutex::new(Some(kill_shard_tx))),
            kill_shard_rx: Arc::new(TokioMutex::new(kill_shard_rx)),
            last_ack: Instant::now(),
            last_heartbeat: Instant::now(),
            connect_time: Instant::now(), // will be overwritten
            ready_tx: Mutex::new(ready_tx),
            ready_guild_count: 0,
            received_count: 0,
            is_ready: false,
            #[cfg(feature = "resume-after-identify")]
            used_resume: false,
            shutdown_rx,
            #[cfg(feature = "whitelabel")]
            command_rx: Arc::new(TokioMutex::new(command_rx)),
            #[cfg(feature = "whitelabel")]
            database,
            event_forwarder,
        }
    }

    #[tracing::instrument(skip(self, resume_data), fields(shard_id = self.get_shard_id(), bot_id = %self.user_id))]
    pub async fn connect(
        mut self,
        resume_data: Option<SessionData>,
    ) -> Result<Option<SessionData>> {
        // TODO: Move to constructor
        self.session_data = resume_data;

        // Build gateway URL
        let gateway_url = self
            .session_data
            .as_ref()
            .and_then(|s| s.resume_url.clone())
            .unwrap_or_else(|| DEFAULT_GATEWAY_URL.to_string());

        let mut uri = format!("{gateway_url}?v={GATEWAY_VERSION}&encoding=json");
        if cfg!(feature = "compression") {
            uri.push_str("&compress=zlib-stream");
        }

        let uri = match Url::parse(&uri[..]) {
            Ok(uri) => uri,
            Err(e) => {
                error!(error = %e, uri = %uri, "Error parsing gateway URI");

                let fallback_url =
                    format!("{DEFAULT_GATEWAY_URL}?v={GATEWAY_VERSION}&encoding=json");
                Url::parse(fallback_url.as_str()).expect("Error parsing fallback gateway URI")
            }
        };

        // If we can RESUME, we can connect straight away
        if self.session_data.is_none() {
            if let Err(e) = self.wait_for_ratelimit().await {
                error!(error = %e, "Error waiting for ratelimit, reconnecting");
                return Err(e);
            }
        }

        info!(%uri, "Connecting to gateway");

        let (wss, _) = connect_async(uri).await?;
        let (ws_tx, ws_rx) = wss.split();
        self.connect_time = Instant::now();

        debug!("Connected to gateway");

        // start writer
        tokio::spawn(handle_writes(
            ws_tx,
            self.writer_rx.take().expect("writer_rx is None"),
        ));

        // start read loop
        self.listen(ws_rx).await?;

        Ok(self.session_data.take())
    }

    // helper function
    async fn write<U: Serialize>(&self, msg: U, tx: oneshot::Sender<Result<()>>) -> Result<()> {
        let message = OutboundMessage::new(msg, tx)?;
        self.writer.send(message).await?;
        Ok(())
    }

    // helper function
    pub fn kill(&self) {
        let kill_shard_tx = self.kill_shard_tx.lock().take();
        let shard_id = self.get_shard_id();

        tokio::spawn(async move {
            match kill_shard_tx {
                Some(kill_shard_tx) => {
                    if kill_shard_tx.send(()).is_err() {
                        warn!(shard_id = %shard_id, "Failed to kill, receiver already unallocated");
                    }
                }
                None => warn!(shard_id = %shard_id, "Tried to kill but kill_shard_tx was None"),
            }
        });
    }

    #[tracing::instrument(name = "listen", skip(self, rx), fields(shard_id = self.get_shard_id(), bot_id = %self.user_id))]
    async fn listen(
        &mut self,
        mut rx: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    ) -> Result<()> {
        #[cfg(feature = "compression")]
        let mut decoder = Decompress::new(true);

        let kill_shard_rx = Arc::clone(&self.kill_shard_rx);
        let kill_shard_rx = &mut *kill_shard_rx.lock().await;

        #[cfg(feature = "whitelabel")]
        let command_rx = Arc::clone(&self.command_rx);
        #[cfg(feature = "whitelabel")]
        let command_rx = &mut *command_rx.lock().await;

        let heartbeat_rx = &mut self
            .heartbeat_rx
            .take()
            .ok_or_else(|| GatewayError::custom("heartbeat_rx is None"))?;
        let mut has_done_heartbeat = false;

        debug!("Starting read loop");
        loop {
            #[cfg(feature = "whitelabel")]
            let command_rx = async { command_rx.recv().await };

            #[cfg(not(feature = "whitelabel"))]
            let command_rx = future::pending::<InternalCommand>();

            tokio::select! {
                // handle kill
                _ = &mut *kill_shard_rx => {
                    info!("Received kill message");
                    break;
                }

                session_tx = self.shutdown_rx.recv() => {
                    info!("Received shutdown message");

                    match session_tx {
                        Ok(session_tx) => {
                            let identifier = ShardIdentifier::new(self.user_id, self.get_shard_id());
                            if let Err(e) = session_tx.send((identifier, self.session_data.take())).await {
                                error!(shard_id = %self.get_shard_id(), error = %e, "Error sending session data");
                            }

                            info!("Reported session data");
                        }
                        Err(e) => error!(shard_id = %self.get_shard_id(), error = %e, "Error receiving shutdown message"),
                    }

                    break;
                }

                _ = heartbeat_rx.recv() => {
                    let elapsed = self.last_ack.checked_duration_since(self.last_heartbeat);
                    debug!(previous_ack_time = ?elapsed.map(|e| e.as_millis()), "Need to send heartbeat");

                    if has_done_heartbeat && (elapsed.is_none() || elapsed.unwrap() > self.heartbeat_interval) {
                        error!("Haven't received heartbeat ack, killing");
                        if let Some(tx) = self.kill_shard_tx.lock().take() {
                            if let Err(e) = tx.send(()) {
                                error!(error = ?e, "Error sending kill notification to shard");
                            }
                        }

                        break;
                    }

                    if let Err(e) = self.do_heartbeat().await { // TODO: Don't block
                        error!(error = %e, "Error sending heartbeat");
                        self.kill();
                        break;
                    }

                    has_done_heartbeat = true;
                }

                // handle incoming payload
                payload = rx.next() => {
                    match payload {
                        None => {
                            warn!("Received None from websocket, killing");
                            self.kill();
                            break;
                        }

                        Some(Err(e)) => {
                            warn!(error = %e, "Error reading data from websocket, killing");
                            self.kill();
                            break;
                        }

                        Some(Ok(Message::Close(frame))) => {
                            info!(?frame, "Got close from gateway");
                            self.kill();

                            if let Some(frame) = frame {
                                match frame.code {
                                    // On code 1000, the session is invalidated
                                    CloseCode::Normal => {
                                        self.session_data = None;
                                    },

                                    CloseCode::Library(code) => {
                                        let close_event = CloseEvent::new(code, frame.reason.to_string());

                                        if !close_event.should_reconnect() {
                                            return GatewayError::AuthenticationError {
                                                bot_token: self.identify.data.token.clone(),
                                                data: close_event,
                                            }.into();
                                        }
                                    },

                                    _ => {},
                                }
                            }

                            break;
                        }

                        Some(Ok(Message::Text(data))) => {
                            trace!(data = %data.as_str(), "Received payload");

                            self.process_payload(data).await
                        }

                        #[cfg(feature = "compression")]
                        Some(Ok(Message::Binary(data))) => {
                            let data = match self.decompress(data, &mut decoder).await {
                                Ok(data) => data,
                                Err(e) => {
                                    error!(error = %e, "Error decompressing payload");
                                    continue;
                                }
                            };

                            let value: Value = serde_json::from_slice(data.as_bytes())?;

                            if let Err(e) = Arc::clone(&self).process_payload(value, data.as_bytes()).await {
                                self.log_err("An error occurred while processing a payload", &e);
                            }
                        }

                        _ => {}
                    }
                }

                // handle internal commands
                command = command_rx => {
                    #[cfg(feature = "whitelabel")]
                    {
                        let Some(command) = command else {continue};

                        match command {
                            InternalCommand::StatusUpdate { status } => {
                                let payload = PresenceUpdate::new(status);

                                let (tx, rx) = oneshot::channel();

                                if let Err(e) = self.write(payload, tx).await {
                                    error!(error = %e, "Error writing presence update payload");
                                    continue;
                                }

                                match rx.await {
                                    Ok(Err(e)) => error!(shard_id = %self.get_shard_id(), error = %e, "Error writing presence update payload"),
                                    Err(e) => error!(shard_id = %self.get_shard_id(), error = %e, "Error writing presence update payload"),
                                    _ => {}
                                }
                            }
                            InternalCommand::Shutdown => {
                                info!("Received shutdown command (via internal command)");
                                break;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    #[cfg(feature = "compression")]
    async fn decompress(
        self: Arc<Self>,
        data: Vec<u8>,
        decoder: &mut Decompress,
    ) -> Result<Vec<u8>> {
        let mut total_rx = self.total_rx.lock();

        let mut output: Vec<u8> = Vec::with_capacity(CHUNK_SIZE);
        let before = total_rx.clone();
        let mut offset: usize = 0;

        while ((decoder.total_in() - *total_rx) as usize) < data.len() {
            let mut temp: Vec<u8> = Vec::with_capacity(CHUNK_SIZE);

            match decoder
                .decompress_vec(&data[offset..], &mut temp, FlushDecompress::Sync)
                .map_err(GatewayError::DecompressError)
            {
                Ok(Status::StreamEnd) => break,
                Ok(Status::Ok) | Ok(Status::BufError) => {
                    output.append(&mut temp);
                    offset = (decoder.total_in() - before) as usize;
                }

                // TODO: Should we reconnect?
                Err(e) => {
                    *total_rx = 0;
                    decoder.reset(true);

                    return Err(e);
                }
            };
        }

        *total_rx = decoder.total_in();

        return Ok(output);
    }

    #[tracing::instrument(skip(self, raw))]
    async fn process_payload(&mut self, raw: String) {
        let opcode = match find_opcode(raw.as_str()) {
            Some(v) => v,
            None => {
                error!(raw = %raw, "Missing opcode");
                return;
            }
        };

        if let Some(seq) = find_seq(raw.as_str()) {
            if let Some(ref mut session_data) = &mut self.session_data {
                session_data.seq = seq; // TODO: Verify this works
            }
        }

        match opcode {
            Opcode::Dispatch => {
                let dispatch = match serde_json::from_str(raw.as_str()) {
                    Ok(dispatch) => dispatch,
                    Err(e) => {
                        error!(error = %e, raw = %raw, "Error deserializing dispatch payload");
                        return;
                    }
                };

                if let Err(e) = self.handle_event(dispatch, raw).await {
                    error!(error = %e, "Error handling dispatch event");
                }
            }

            Opcode::Reconnect => {
                info!("Received reconnect payload from Discord");
                self.kill();
            }

            Opcode::InvalidSession => {
                info!("Received invalid session payload from Discord");

                self.session_data = None;
                self.kill();
            }

            Opcode::Hello => {
                let hello: payloads::Hello = match serde_json::from_str(raw.as_str()) {
                    Ok(hello) => hello,
                    Err(e) => {
                        error!(error = %e, raw = %raw, "Error deserializing HELLO payload");
                        return;
                    }
                };

                let interval = Duration::from_millis(hello.data.heartbeat_interval as u64);
                self.heartbeat_interval = interval;

                let resume_info = self.session_data.clone();

                info!(
                    heartbeat_interval = self.heartbeat_interval.as_millis(),
                    has_resume_info = resume_info.is_some(),
                    "Received HELLO payload"
                );

                if let Some(resume_info) = resume_info {
                    info!(resume_url = ?resume_info.resume_url, seq = %resume_info.seq, session_id = %resume_info.session_id, "Attempting resume");

                    if let Err(e) = self
                        .do_resume(resume_info.session_id, resume_info.seq)
                        .await
                    {
                        error!(error = %e, "Error sending RESUME payload, killing");
                        self.kill();
                        return;
                    }

                    #[cfg(feature = "resume-after-identify")]
                    {
                        self.used_resume = true;
                    }
                } else {
                    info!("No resume info found, IDENTIFYing instead");

                    if self.connect_time.elapsed() > interval {
                        info!(
                            time_since_connect = self.connect_time.elapsed().as_millis(),
                            "Connected over 45s ago, Discord will kick us off. Reconnecting."
                        );
                        self.kill();
                        return;
                    }

                    if self.connect_time.elapsed() > Duration::from_millis(500) {
                        debug!("Connected over 500ms ago, waiting for ratelimit");
                        if let Err(e) = self.wait_for_ratelimit().await {
                            error!(error = %e, "Error waiting for ratelimit, reconnecting");
                            self.kill();
                            return;
                        }
                    }

                    if let Err(e) = self.do_identify().await {
                        error!(error = %e, "Error sending IDENTIFY payload, killing");
                        self.kill();
                        return;
                    }

                    info!("Identified");

                    if let Err(e) = self.update_ratelimit_after_identify().await {
                        error!(error = %e, "Error setting identify ratelimit value");
                    }
                }

                timer(self.heartbeat_tx.clone(), interval, false);
            }

            Opcode::HeartbeatAck => {
                trace!("Received heartbeat ack");
                self.last_ack = Instant::now();
            }

            _ => {}
        }
    }

    #[tracing::instrument(skip(self, payload, data), fields(event_type = %payload.data))]
    async fn handle_event(&mut self, payload: Dispatch, data: String) -> Result<()> {
        // Gateway events
        match &payload.data {
            Event::Ready(ready) => {
                self.session_data = Some(SessionData {
                    seq: payload.seq,
                    session_id: ready.session_id.clone(),
                    resume_url: Some(ready.resume_gateway_url.clone()),
                });

                self.ready_guild_count = ready.guilds.len() as u16;

                info!(
                    guild_count = ready.guilds.len(),
                    username = ready.user.username,
                    "Got READY event"
                );

                return Ok(());
            }

            Event::Resumed(_) => {
                info!("Received RESUME acknowledgement");

                if !self.is_ready {
                    self.is_ready = true;

                    if let Some(tx) = self.ready_tx.lock().take() {
                        if tx.send(()).is_err() {
                            error!("Ready notification received hung up, can't send ready notification");
                        }
                    }
                }

                return Ok(());
            }

            Event::GuildCreate(g) => {
                self.increment_received_count().await;

                #[cfg(feature = "whitelabel")]
                if let Err(e) = self.store_whitelabel_guild(g.id).await {
                    error!(error = %e, "Error storing whitelabel guild data");
                }
            }

            _ => {}
        }

        if is_whitelisted(&payload.data) {
            let guild_id: Option<Snowflake> = super::event_forwarding::get_guild_id(&payload.data);

            let raw_payload = match RawValue::from_string(data) {
                Ok(v) => v,
                Err(e) => {
                    error!(error = %e, "Error convering JSON string to RawValue");
                    return Err(e.into());
                }
            };

            // prepare payload
            let wrapped = event_forwarding::Event {
                bot_token: self.identify.data.token.clone(),
                bot_id: self.user_id.0,
                is_whitelabel: is_whitelabel(),
                shard_id: self.get_shard_id(),
                event: raw_payload,
            };

            if let Err(e) = self
                .event_forwarder
                .forward_event(&self.config, wrapped, guild_id)
                .await
            {
                error!(error = %e, "Error while executing worker HTTP request");
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn increment_received_count(&mut self) {
        self.received_count += 1;

        if !self.is_ready {
            // Once we have 90% of the guilds, we're ok to load more shards
            if self.received_count >= (self.ready_guild_count / 100 * 90).into() {
                self.is_ready = true;

                if let Some(tx) = self.ready_tx.lock().take() {
                    info!(received_count = self.received_count, "Reporting readiness");
                    if tx.send(()).is_err() {
                        error!("Error sending ready notification to probe (receiver hung up)");
                    }
                    info!("Reported readiness");
                }

                #[cfg(feature = "resume-after-identify")]
                if !self.used_resume {
                    let kill_shard_tx = self.kill_shard_tx.lock().take();

                    info!(
                        received_count = self.received_count,
                        "Received 90% of guilds, going to killing shard in 90 seconds"
                    );
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_secs(90)).await;
                        info!("Received 90% of guilds, killing shard");

                        match kill_shard_tx {
                            Some(kill_shard_tx) => {
                                if kill_shard_tx.send(()).is_err() {
                                    warn!("Failed to kill, receiver already unallocated");
                                }
                            }
                            None => {
                                warn!("Tried to kill but kill_shard_tx was None")
                            }
                        }
                    });
                }
            }
        }
    }

    fn meets_forward_threshold(&self, event: &Event) -> bool {
        if cfg!(feature = "skip-initial-guild-creates") {
            if let Event::GuildCreate(_) = event {
                // if not ready, don't forward event
                return self.is_ready;
            }
        }

        true
    }

    #[tracing::instrument(skip(self))]
    async fn do_heartbeat(&mut self) -> Result<()> {
        debug!("Sending heartbeat");

        let seq = self.session_data.as_ref().map(|s| s.seq);
        let payload = payloads::Heartbeat::new(seq);

        let (tx, rx) = oneshot::channel();
        self.write(payload, tx).await?;

        rx.await??;

        self.last_heartbeat = Instant::now();
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn do_identify(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.write(&self.identify, tx).await?;

        Ok(rx.await??)
    }

    #[tracing::instrument(skip(self))]
    async fn wait_for_ratelimit(&self) -> Result<()> {
        info!("Waiting for IDENTIFY ratelimit");

        let key = self.get_ratelimit_key();

        let mut res = redis::Value::Nil;
        while res == redis::Value::Nil {
            let mut conn = self.redis.get().await?;

            res = cmd("SET")
                .arg(&[&key[..], "1", "NX", "PX", "8000"]) // some arbitrary value, set if not exist, set expiry, of 8s to allow websocket connection
                .query_async(&mut conn)
                .await?;

            if res == redis::Value::Nil {
                // get time to delay
                let ttl = cmd("PTTL").arg(&key).query_async(&mut conn).await?;

                if let redis::Value::Int(ttl) = ttl {
                    // if number is negative, we can go ahead and identify
                    // -1 = no expire, -2 = doesn't exist
                    if ttl > 0 {
                        let ttl = Duration::from_millis(ttl as u64);
                        sleep(ttl).await
                    }
                }
            }
        }

        info!("Got IDENTIFY ratelimit token");
        Ok(())
    }

    async fn update_ratelimit_after_identify(&self) -> Result<()> {
        let key = self.get_ratelimit_key();
        self.redis_write(key.as_str(), "1", Some(5)).await
    }

    fn get_ratelimit_key(&self) -> String {
        if is_whitelabel() {
            format!("ratelimiter:whitelabel:identify:{}", self.user_id)
        } else {
            format!(
                "ratelimiter:public:identify:{}",
                self.get_shard_id() % self.large_sharding_buckets
            )
        }
    }

    /// Shard.session_id & Shard.seq should not be None when calling this function
    /// if they are, the function will panic
    #[tracing::instrument(skip(self))]
    async fn do_resume(&self, session_id: String, seq: usize) -> Result<()> {
        let payload = payloads::Resume::new(self.identify.data.token.clone(), session_id, seq);

        let (tx, rx) = oneshot::channel();
        self.write(payload, tx).await?;

        Ok(rx.await??)
    }

    #[tracing::instrument(skip(self, value, expiry))]
    async fn redis_write<U: ToString>(
        &self,
        key: &str,
        value: U,
        expiry: Option<usize>,
    ) -> Result<()> {
        let mut conn = self.redis.get().await?;
        match expiry {
            Some(expiry) => conn.set_ex(key, value.to_string(), expiry).await?,
            None => conn.set(key, value.to_string()).await?,
        }

        Ok(())
    }

    /// helper
    pub fn get_shard_id(&self) -> u16 {
        self.identify.data.shard_info.shard_id
    }
}

async fn handle_writes(
    mut tx: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    mut rx: mpsc::Receiver<super::OutboundMessage>,
) {
    while let Some(msg) = rx.recv().await {
        let payload = Message::text(msg.message);
        let res = tx.send(payload).await;

        if let Err(e) = msg.tx.send(res.map_err(|e| e.into())) {
            error!(error = ?e, "Error while sending write result back to caller");
        }
    }
}
