use super::payloads;
use super::payloads::event::Event;
use super::payloads::{Opcode, Payload, Dispatch};

use super::OutboundMessage;

use flate2::{Decompress, FlushDecompress, Status};

use tokio::time::delay_for;
use std::time::Duration;
use std::time::Instant;

use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use std::sync::atomic::{AtomicU16, Ordering};

use tokio::sync::{mpsc, oneshot};
use futures_util::{StreamExt, SinkExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use serde::Serialize;
use cache::{PostgresCache, Cache};
use crate::gateway::GatewayError;
use model::user::StatusUpdate;
use std::fmt::Display;
use std::str;

use tokio_tungstenite::WebSocketStream;
use futures_util::stream::{SplitStream, SplitSink};
use tokio::net::TcpStream;
use tokio_tls::TlsStream;
use common::event_forwarding;
use model::guild::{Member, Guild};
use crate::gateway::payloads::PresenceUpdate;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use deadpool_redis::{Pool, cmd};
use serde_json::value::RawValue;
use model::Snowflake;
use crate::config::Config;

type WebSocketTx = SplitSink<WebSocketStream<tokio_tungstenite::stream::Stream<TcpStream, TlsStream<TcpStream>>>, Message>;
type WebSocketRx = SplitStream<WebSocketStream<tokio_tungstenite::stream::Stream<TcpStream, TlsStream<TcpStream>>>>;

pub struct Shard {
    pub(crate) config: Arc<Config>,
    identify: payloads::Identify,
    large_sharding_buckets: u16,
    cache: Arc<PostgresCache>,
    redis: Arc<Pool>,
    is_whitelabel: bool,
    pub status_update_tx: mpsc::Sender<StatusUpdate>,
    status_update_rx: Mutex<mpsc::Receiver<StatusUpdate>>,
    user_id: Snowflake,
    total_rx: Mutex<u64>,
    seq: RwLock<Option<usize>>,
    session_id: RwLock<Option<String>>,
    writer: RwLock<Option<mpsc::Sender<OutboundMessage>>>,
    kill_heartbeat: Mutex<Option<oneshot::Sender<()>>>,
    pub kill_shard_tx: Mutex<Option<oneshot::Sender<()>>>,
    kill_shard_rx: Mutex<oneshot::Receiver<()>>,
    last_ack: RwLock<Instant>,
    last_heartbeat: RwLock<Instant>,
    connect_time: RwLock<Instant>,
    ready_tx: Mutex<Option<mpsc::Sender<u16>>>,
    ready_guild_count: AtomicU16,
    received_count: AtomicU16,
    is_ready: RwLock<bool>,
    pub(crate) http_client: reqwest::Client,
    pub(crate) cookie: RwLock<Option<Box<str>>>,
}

// 16 KiB
const CHUNK_SIZE: usize = 16 * 1024;

impl Shard {
    pub fn new(
        config: Arc<Config>,
        identify: payloads::Identify,
        large_sharding_buckets: u16,
        cache: Arc<PostgresCache>,
        redis: Arc<Pool>,
        is_whitelabel: bool,
        user_id: Snowflake,
        ready_tx: Option<mpsc::Sender<u16>>,
    ) -> Arc<Shard> {
        let (kill_shard_tx, kill_shard_rx) = oneshot::channel();
        let (status_update_tx, status_update_rx) = mpsc::channel(1);

        let shard = Arc::new(Shard {
            config,
            identify,
            large_sharding_buckets,
            cache,
            redis,
            is_whitelabel,
            status_update_tx,
            status_update_rx: Mutex::new(status_update_rx),
            user_id,
            total_rx: Mutex::new(0),
            seq: RwLock::new(None),
            session_id: RwLock::new(None),
            writer: RwLock::new(None),
            kill_heartbeat: Mutex::new(None),
            kill_shard_tx: Mutex::new(Some(kill_shard_tx)),
            kill_shard_rx: Mutex::new(kill_shard_rx),
            last_ack: RwLock::new(Instant::now()),
            last_heartbeat: RwLock::new(Instant::now()),
            connect_time: RwLock::new(Instant::now()), // will be overwritten
            ready_tx: Mutex::new(ready_tx),
            ready_guild_count: AtomicU16::new(0),
            received_count: AtomicU16::new(0),
            is_ready: RwLock::new(false),
            http_client: Shard::build_http_client(),
            cookie: RwLock::new(None),
        });

        shard
    }

    pub async fn connect(self: Arc<Self>) -> Result<(), GatewayError> {
        //rst
        let (kill_shard_tx, kill_shard_rx) = oneshot::channel();
        *self.kill_shard_tx.lock().await = Some(kill_shard_tx);
        *self.kill_shard_rx.lock().await = kill_shard_rx;

        *self.total_rx.lock().await = 0;
        self.ready_guild_count.store(0, Ordering::Relaxed);
        self.received_count.store(0, Ordering::Relaxed);
        *self.is_ready.write().await = false;

        *self.last_heartbeat.write().await = Instant::now();
        *self.last_ack.write().await = Instant::now();
        // rst

        let uri = url::Url::parse("wss://gateway.discord.gg/?v=8&encoding=json&compress=zlib-stream").unwrap();

        let (wss, _) = connect_async(uri).await?;
        let (ws_tx, ws_rx) = wss.split();
        *self.connect_time.write().await = Instant::now();

        // start writer
        let (writer_tx, writer_rx) = mpsc::channel(16);
        tokio::spawn(async move {
            Shard::handle_writes(ws_tx, writer_rx).await;
        });
        *self.writer.write().await = Some(writer_tx);

        // start read loop
        if let Err(e) = self.listen(ws_rx).await {
            return Err(e);
        }

        Ok(())
    }

    async fn handle_writes(mut ws_tx: WebSocketTx, mut rx: mpsc::Receiver<super::OutboundMessage>) {
        while let Some(msg) = rx.recv().await {
            let payload = tokio_tungstenite::tungstenite::Message::text(msg.message);
            let res = ws_tx.send(payload).await;

            if let Err(e) = msg.tx.send(res) {
                eprintln!("Error while sending write result back to caller: {:?}", e);
            }
        }
    }

    // helper function
    async fn write<T: Serialize>(
        &self,
        msg: T,
        tx: oneshot::Sender<Result<(), tokio_tungstenite::tungstenite::Error>>,
    ) -> Result<(), GatewayError> {
        OutboundMessage::new(msg, tx)
            .map_err(GatewayError::JsonError)?
            .send(self.writer.read().await.clone().unwrap())
            .await
            .map_err(GatewayError::SendMessageError)?;

        Ok(())
    }

    // helper function
    pub fn kill(self: Arc<Self>) {
        // BIG problem
        // TODO: Make this good
        tokio::spawn(async move {
            // TODO: panic?
            let kill_shard_tx = self.kill_shard_tx.lock().await.take();
            let kill_heartbeat_tx = self.kill_heartbeat.lock().await.take();

            match kill_shard_tx {
                Some(kill_shard_tx) => {
                    if let Err(_) = kill_shard_tx.send(()) {
                        self.log_err("Failed to kill", &GatewayError::custom("Receiver already unallocated"));
                    }
                }
                None => self.log("Tried to kill but kill_shard_tx was None")
            }

            match kill_heartbeat_tx {
                Some(kill_heartbeat_tx) => {
                    if let Err(_) = kill_heartbeat_tx.send(()) {
                        self.log_err("Failed to kill heartbeat", &GatewayError::custom("Receiver already unallocated"));
                    }
                }
                None => self.log("Tried to kill but kill_heartbeat_tx was None")
            }
        });
    }

    async fn listen(self: Arc<Self>, mut ws_rx: WebSocketRx) -> Result<(), GatewayError> {
        let mut decoder = Decompress::new(true);

        loop {
            let shard = Arc::clone(&self);
            let kill_rx = &mut *shard.kill_shard_rx.lock().await;
            let status_update_rx = &mut shard.status_update_rx.lock().await;

            tokio::select! {
                // handle kill
                _ = kill_rx => {
                    self.log("Received kill message");
                    break;
                }

                // handle incoming payload
                payload = ws_rx.next() => {
                    match payload {
                        None => {
                            self.log("Payload was None, killing");
                            self.kill();
                            break;
                        }

                        Some(Err(e)) => {
                            self.log_err("Error reading data from websocket, killing", &GatewayError::WebsocketError(e));
                            self.kill();
                            break;
                        }

                        Some(Ok(Message::Close(frame))) => {
                            self.log(format!("Got close from gateway: {:?}", frame));
                            Arc::clone(&self).kill();

                            if let Some(frame) = frame {
                                if let CloseCode::Library(code) = frame.code {
                                    let fatal_codes: [u16; 2] = [4004, 4014];
                                    if fatal_codes.contains(&code) {
                                        return GatewayError::AuthenticationError {
                                            bot_token: self.identify.data.token.clone(),
                                            error_code: frame.code,
                                            error: frame.reason.to_string(),
                                        }.into();
                                    }
                                }
                            }

                            break;
                        }

                        Some(Ok(Message::Binary(data))) => {
                            let (payload, data) = match Arc::clone(&self).read_payload(data, &mut decoder).await {
                                Some(r) => r,
                                _ => continue
                            };

                            if let Err(e) = Arc::clone(&self).process_payload(payload, data).await {
                                self.log_err("An error occurred while processing a payload", &e);
                            }
                        }

                        _ => {}
                    }
                }

                // handle status update
                presence = status_update_rx.recv() => {
                    if let Some(presence) = presence {
                        let (tx, rx) = oneshot::channel();

                        let shard = Arc::clone(&self);
                        tokio::spawn(async move {
                            let payload = PresenceUpdate::new(presence);
                            if let Err(e) = shard.write(payload, tx).await {
                                shard.log_err("Error sending presence update payload to writer", &e);
                            }

                            match rx.await {
                                Ok(Err(e)) => shard.log_err("Error writing presence update payload", &GatewayError::WebsocketError(e)),
                                Err(e) => shard.log_err("Error writing presence update payload", &GatewayError::RecvError(e)),
                                _ => {}
                            }
                        });
                    }
                }
            }
        }

        Ok(())
    }

    // we return None because it's ok to discard the payload
    async fn read_payload(self: Arc<Self>, data: Vec<u8>, decoder: &mut Decompress) -> Option<(Payload, Vec<u8>)> {
        let mut total_rx = self.total_rx.lock().await;

        let mut output: Vec<u8> = Vec::with_capacity(CHUNK_SIZE);
        let before = total_rx.clone();
        let mut offset: usize = 0;

        while ((decoder.total_in() - *total_rx) as usize) < data.len() {
            let mut temp: Vec<u8> = Vec::with_capacity(CHUNK_SIZE);

            match decoder.decompress_vec(&data[offset..], &mut temp, FlushDecompress::Sync).map_err(GatewayError::DecompressError) {
                Ok(Status::StreamEnd) => break,
                Ok(Status::Ok) | Ok(Status::BufError) => {
                    output.append(&mut temp);
                    offset = (decoder.total_in() - before) as usize;
                }

                // TODO: Should we reconnect?
                Err(e) => {
                    self.log_err("Error while decompressing", &e);

                    *total_rx = 0;
                    decoder.reset(true);

                    return None;
                }
            };
        }

        *total_rx = decoder.total_in();

        // deserialize payload
        match serde_json::from_slice(&output[..]).map_err(GatewayError::JsonError) {
            Ok(payload) => Some((payload, output)),
            Err(e) => {
                self.log_err("Error while deserializing payload", &e);
                None
            }
        }
    }

    async fn process_payload(self: Arc<Self>, payload: Payload, raw: Vec<u8>) -> Result<(), GatewayError> {
        if let Some(seq) = payload.seq {
            *self.seq.write().await = Some(seq);

            if let Err(e) = self.save_seq().await {
                self.log_err("Error saving sequence number", &e);
            }
        }

        match payload.opcode {
            Opcode::Dispatch => {
                let payload = serde_json::from_slice(&raw[..])?;

                if let Err(e) = Arc::clone(&self).handle_event(payload).await {
                    self.log_err("Error processing dispatch", &e);
                }
            }

            Opcode::Reconnect => {
                self.log("Received reconnect payload from Discord");
                self.kill();
            }

            Opcode::InvalidSession => {
                self.log("Received invalid session payload from Discord");

                *self.session_id.write().await = None;
                *self.seq.write().await = None;

                // delete session ID from Redis
                if let Err(e) = self.delete_session_id().await {
                    self.log_err("Error deleting session_id from Redis", &e);
                }

                // delete seq from Redis
                if let Err(e) = self.delete_seq().await {
                    self.log_err("Error deleting seq from Redis", &e);
                }

                self.kill();
            }

            Opcode::Hello => {
                let hello: payloads::Hello = serde_json::from_slice(&raw[..])?;
                let interval = Duration::from_millis(hello.data.heartbeat_interval as u64);

                let mut should_identify = true;

                // try to load session_id from redis
                if let Ok(Some(session_id)) = self.load_session_id().await {
                    *self.session_id.write().await = Some(session_id);

                    // if success, load seq from redis
                    if let Ok(Some(seq)) = self.load_seq().await {
                        *self.seq.write().await = Some(seq)
                    }
                }

                if let (Some(session_id), Some(seq)) = (self.session_id.read().await.as_ref().cloned(), *self.seq.read().await) {
                    if let Err(e) = Arc::clone(&self).do_resume(session_id.clone(), seq).await {
                        self.log_err("Error RESUMEing, going to IDENTIFY", &e);

                        // rst
                        *self.session_id.write().await = None;
                        *self.seq.write().await = None;

                        self.wait_for_ratelimit().await?;

                        if self.connect_time.read().await.elapsed() > interval {
                            self.log("Connected over 45s ago, Discord will kick us off. Reconnecting.");
                            Arc::clone(&self).kill();
                            return Ok(());
                        } else {
                            should_identify = Arc::clone(&self).do_identify().await.is_err();
                        }
                    } else {
                        should_identify = false;
                        self.log("Sent resume successfully");
                    }
                }

                if should_identify {
                    self.wait_for_ratelimit().await?;

                    if self.connect_time.read().await.elapsed() > interval {
                        self.log("Connected over 45s ago, Discord will kick us off. Reconnecting.");
                        self.kill();
                        return Ok(());
                    }

                    if let Err(e) = Arc::clone(&self).do_identify().await {
                        self.log_err("Error identifying, killing", &e);
                        self.kill();
                        return e.into();
                    }

                    self.log("Identified");
                }

                let kill_tx = Arc::clone(&self).start_heartbeat(interval).await;
                *self.kill_heartbeat.lock().await = Some(kill_tx)
            }

            Opcode::HeartbeatAck => {
                *self.last_ack.write().await = Instant::now();

                // save session ID
                if let Err(e) = self.save_session_id().await {
                    self.log_err("Error occurred while saving session ID", &e);
                }
            }

            _ => {}
        }

        Ok(())
    }

    async fn handle_event(self: Arc<Self>, data: Box<RawValue>) -> Result<(), GatewayError> {
        let payload: Dispatch = serde_json::from_str(data.get()).map_err(GatewayError::JsonError)?;

        // Gateway events
        match &payload.data {
            Event::Ready(ready) => {
                *self.session_id.write().await = Some(ready.session_id.clone());
                if let Err(e) = self.save_session_id().await {
                    self.log_err("Error saving session ID to Redis", &e);
                }

                self.ready_guild_count.store(ready.guilds.len() as u16, Ordering::Relaxed);

                self.log(format!("Ready on {}#{} ({})", ready.user.username, ready.user.discriminator, ready.user.id));
                return Ok(());
            }

            Event::Resumed(_) => {
                self.log("Received resumed acknowledgement");

                if !*self.is_ready.read().await {
                    if let Some(mut tx) = self.ready_tx.lock().await.take() {
                        if let Err(e) = tx.send(self.get_shard_id()).await.map_err(GatewayError::SendU16Error) {
                            self.log_err("Error sending ready notification to probe", &e);
                        }
                    }
                }

                return Ok(());
            }

            Event::GuildCreate(_) => {
                if !*self.is_ready.read().await {
                    let received = self.received_count.fetch_add(1, Ordering::Relaxed);
                    if received >= (self.ready_guild_count.load(Ordering::Relaxed) / 100) * 90 { // Once we have 90% of the guilds, we're ok to load more shards
                        if let Some(mut tx) = self.ready_tx.lock().await.take() {
                            if let Err(e) = tx.send(self.get_shard_id()).await.map_err(GatewayError::SendU16Error) {
                                self.log_err("Error sending ready notification to probe", &e);
                            }
                        }
                    }
                }
            }

            _ => {}
        }

        // cache + push to redis
        tokio::spawn(async move {
            let guild_id = Shard::get_guild_id(&payload.data);

            // cache
            let res = match payload.data {
                Event::ChannelCreate(channel) => self.cache.store_channel(channel).await,
                Event::ChannelUpdate(channel) => self.cache.store_channel(channel).await,
                Event::ChannelDelete(channel) => self.cache.delete_channel(channel.id).await,
                Event::GuildCreate(mut guild) => {
                    Shard::apply_guild_id_to_channels(&mut guild);
                    self.cache.store_guild(guild).await
                }
                Event::GuildUpdate(mut guild) => {
                    Shard::apply_guild_id_to_channels(&mut guild);
                    self.cache.store_guild(guild).await
                }
                Event::GuildDelete(guild) => {
                    if guild.unavailable.is_none() { // we were kicked
                        self.cache.delete_guild(guild.id).await
                    } else {
                        Ok(())
                    }
                }
                Event::GuildBanAdd(ev) => self.cache.delete_member(ev.user.id, ev.guild_id).await,
                Event::GuildEmojisUpdate(ev) => self.cache.store_emojis(ev.emojis, ev.guild_id).await,
                Event::GuildMemberAdd(ev) => self.cache.store_member(ev.member, ev.guild_id).await,
                Event::GuildMemberRemove(ev) => self.cache.delete_member(ev.user.id, ev.guild_id).await,
                Event::GuildMemberUpdate(ev) => self.cache.store_member(Member {
                    user: Some(ev.user),
                    nick: ev.nick,
                    roles: ev.roles,
                    joined_at: ev.joined_at,
                    premium_since: ev.premium_since,
                    deaf: false, // TODO: Don't update these fields somehow?
                    mute: false, // TODO: Don't update these fields somehow?
                }, ev.guild_id).await,
                Event::GuildMembersChunk(ev) => self.cache.store_members(ev.members, ev.guild_id).await,
                Event::GuildRoleCreate(ev) => self.cache.store_role(ev.role, ev.guild_id).await,
                Event::GuildRoleUpdate(ev) => self.cache.store_role(ev.role, ev.guild_id).await,
                Event::GuildRoleDelete(ev) => self.cache.delete_role(ev.role_id).await,
                Event::UserUpdate(user) => self.cache.store_user(user).await,
                _ => Ok(()),
            };

            if let Err(e) = res {
                self.log_err("Error updating cache", &GatewayError::CacheError(e));
            }

            // push to workers, even if error occurred
            // prepare payload
            let wrapped = event_forwarding::Event {
                bot_token: &self.identify.data.token[..],
                bot_id: self.user_id.0,
                is_whitelabel: self.is_whitelabel,
                shard_id: self.get_shard_id(),
                event: &data,
            };

            match Arc::clone(&self).forward_event(wrapped, guild_id).await {
                Ok(res) => {
                    if let Some(error) = res.error {
                        self.log_err("Error occurred while forwarding event to worker", &GatewayError::WorkerError(error))
                    }
                }
                Err(e) => self.log_err("Error while executing worker HTTP request", &e),
            }
        });

        Ok(())
    }

    fn apply_guild_id_to_channels(guild: &mut Guild) {
        if let Some(channels) = &mut guild.channels {
            for channel in channels {
                channel.guild_id = Some(guild.id)
            }
        }
    }

    // returns cancellation channel
    async fn start_heartbeat(
        self: Arc<Self>,
        interval: Duration,
    ) -> oneshot::Sender<()> {
        let (cancel_tx, mut cancel_rx) = oneshot::channel();

        tokio::spawn(async move {
            delay_for(interval).await;

            let mut has_done_heartbeat = false;
            while let Err(oneshot::error::TryRecvError::Empty) = cancel_rx.try_recv() {
                let shard = Arc::clone(&self);
                let elapsed = shard.last_ack.read().await.checked_duration_since(*self.last_heartbeat.read().await);

                if has_done_heartbeat && (elapsed.is_none() || elapsed.unwrap() > interval) {
                    shard.log("Hasn't received heartbeat ack, killing");
                    shard.kill();
                    break;
                }

                if let Err(e) = Arc::clone(&shard).do_heartbeat().await {
                    shard.log_err("Error sending heartbeat, killing", &e);
                    shard.kill();
                    break;
                }

                has_done_heartbeat = true;

                delay_for(interval).await;
            }
        });

        cancel_tx
    }

    async fn do_heartbeat(self: Arc<Self>) -> Result<(), GatewayError> {
        let payload = payloads::Heartbeat::new(*self.seq.read().await);

        let (tx, rx) = oneshot::channel();
        self.write(payload, tx).await?;

        rx.await.map_err(GatewayError::RecvError)?.map_err(GatewayError::WebsocketError)?;

        *self.last_heartbeat.write().await = Instant::now();
        Ok(())
    }

    async fn do_identify(self: Arc<Self>) -> Result<(), GatewayError> {
        let (tx, rx) = oneshot::channel();
        self.write(&self.identify, tx).await?;

        Ok(rx.await.map_err(GatewayError::RecvError)?.map_err(GatewayError::WebsocketError)?)
    }

    async fn wait_for_ratelimit(&self) -> Result<(), GatewayError> {
        let key = if self.is_whitelabel {
            format!("ratelimiter:whitelabel:identify:{}", self.user_id)
        } else {
            format!("ratelimiter:public:identify:{}", self.get_shard_id() % self.large_sharding_buckets)
        };

        let mut res = redis::Value::Nil;
        while res == redis::Value::Nil {
            let mut conn = self.redis.get().await?;

            res = cmd("SET")
                .arg(&[&key[..], "1", "NX", "PX", "6000"]) // some arbitrary value, set if not exist, set expiry, of 6s
                .query_async(&mut conn)
                .await
                .map_err(GatewayError::RedisError)?;

            if res == redis::Value::Nil {
                // get time to delay
                let ttl = cmd("PTTL")
                    .arg(&key)
                    .query_async(&mut conn)
                    .await
                    .map_err(GatewayError::RedisError)?;

                match ttl {
                    redis::Value::Int(ttl) => {
                        // if number is negative, we can go ahead and identify
                        // -1 = no expire, -2 = doesn't exist
                        if ttl > 0 {
                            let ttl = Duration::from_millis(ttl as u64);
                            delay_for(ttl).await
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    /// Shard.session_id & Shard.seq should not be None when calling this function
    /// if they are, the function will panic
    async fn do_resume(self: Arc<Self>, session_id: String, seq: usize) -> Result<(), GatewayError> {
        let payload = payloads::Resume::new(
            self.identify.data.token.clone(),
            session_id,
            seq,
        );

        let (tx, rx) = oneshot::channel();
        self.write(payload, tx).await?;

        Ok(rx.await.map_err(GatewayError::RecvError)?.map_err(GatewayError::WebsocketError)?)
    }

    async fn save_session_id(&self) -> Result<(), GatewayError> {
        match &*self.session_id.read().await {
            Some(session_id) => {
                let mut conn = self.redis.get().await?;

                let key = match self.get_resume_key().await {
                    Some(key) => key,
                    None => return Ok(()),
                };

                cmd("SET")
                    .arg(&[&key[..], session_id, "EX", "120"]) // expiry of 120s
                    .query_async(&mut conn)
                    .await
                    .map_err(GatewayError::RedisError)?;

                Ok(())
            }
            None => Ok(()),
        }
    }

    async fn load_session_id(&self) -> Result<Option<String>, GatewayError> {
        let key = match self.get_resume_key().await {
            Some(key) => key,
            None => return Ok(None),
        };

        let mut conn = self.redis.get().await?;

        let res = cmd("GET")
            .arg(&[&key[..]])
            .query_async(&mut conn)
            .await
            .map_err(GatewayError::RedisError)?;

        match res {
            redis::Value::Data(data) => {
                let session_id = str::from_utf8(&data[..]).map_err(GatewayError::Utf8Error)?.to_owned();
                Ok(Some(session_id))
            }
            _ => Ok(None)
        }
    }

    async fn save_seq(&self) -> Result<(), GatewayError> {
        match &*self.seq.read().await {
            Some(seq) => {
                let mut conn = self.redis.get().await?;

                let key = match self.get_seq_key().await {
                    Some(key) => key,
                    None => return Ok(()),
                };

                cmd("SET")
                    .arg(&[&key[..], &seq.to_string()[..], "EX", "120"]) // expiry of 120s
                    .query_async(&mut conn)
                    .await
                    .map_err(GatewayError::RedisError)?;

                Ok(())
            }
            None => Ok(()),
        }
    }

    async fn load_seq(&self) -> Result<Option<usize>, GatewayError> {
        let key = match self.get_seq_key().await {
            Some(key) => key,
            None => return Ok(None),
        };

        let mut conn = self.redis.get().await?;

        let res = cmd("GET")
            .arg(&[&key[..]])
            .query_async(&mut conn)
            .await
            .map_err(GatewayError::RedisError)?;

        match res {
            redis::Value::Data(data) => {
                match str::from_utf8(&data[..]).map_err(GatewayError::Utf8Error)?.parse() {
                    Ok(seq) => Ok(Some(seq)),
                    Err(_) => Ok(None),
                }
            }
            _ => Ok(None)
        }
    }

    async fn delete_session_id(&self) -> Result<(), GatewayError> {
        let mut conn = self.redis.get().await?;

        let key = match self.get_resume_key().await {
            Some(key) => key,
            None => return Ok(()),
        };

        cmd("DEL")
            .arg(&[&key[..]])
            .query_async(&mut conn)
            .await
            .map_err(GatewayError::RedisError)?;

        Ok(())
    }

    async fn delete_seq(&self) -> Result<(), GatewayError> {
        let mut conn = self.redis.get().await?;

        let key = match self.get_seq_key().await {
            Some(key) => key,
            None => return Ok(()),
        };

        cmd("DEL")
            .arg(&[&key[..]])
            .query_async(&mut conn)
            .await
            .map_err(GatewayError::RedisError)?;

        Ok(())
    }

    async fn get_resume_key(&self) -> Option<String> {
        if self.is_whitelabel {
            Some(format!("tickets:resume:{}:{}", self.user_id, self.get_shard_id()))
        } else {
            Some(format!("tickets:resume:public:{}-{}", self.get_shard_id(), self.identify.data.shard_info.num_shards))
        }
    }

    async fn get_seq_key(&self) -> Option<String> {
        if self.is_whitelabel {
            Some(format!("tickets:seq:{}:{}", self.user_id, self.get_shard_id()))
        } else {
            Some(format!("tickets:seq:public:{}-{}", self.get_shard_id(), self.identify.data.shard_info.num_shards))
        }
    }

    /// helper
    fn get_shard_id(&self) -> u16 {
        self.identify.data.shard_info.shard_id
    }

    pub fn log(&self, msg: impl Display) {
        if self.is_whitelabel {
            println!("[{}] {}", self.user_id, msg);
        } else {
            println!("[{:0>2}] {}", self.get_shard_id(), msg);
        }
    }

    pub fn log_err(&self, msg: impl Display, err: &GatewayError) {
        if self.is_whitelabel {
            eprintln!("[{}] {}: {}", self.user_id, msg, err);
        } else {
            eprintln!("[{:0>2}] {}: {}", self.get_shard_id(), msg, err);
        }
    }
}