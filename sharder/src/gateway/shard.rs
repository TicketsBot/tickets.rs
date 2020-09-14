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
use model::Snowflake;
use crate::gateway::GatewayError;
use crate::manager::FatalError;
use model::user::StatusUpdate;
use std::fmt::Display;

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

type WebSocketTx = SplitSink<WebSocketStream<tokio_tungstenite::stream::Stream<TcpStream, TlsStream<TcpStream>>>, Message>;
type WebSocketRx = SplitStream<WebSocketStream<tokio_tungstenite::stream::Stream<TcpStream, TlsStream<TcpStream>>>>;

pub struct Shard {
    identify: payloads::Identify,
    large_sharding_buckets: u16,
    cache: Arc<PostgresCache>,
    redis: Arc<Pool>,
    is_whitelabel: bool,
    error_tx: mpsc::Sender<FatalError>,
    pub status_update_tx: mpsc::Sender<StatusUpdate>,
    status_update_rx: Mutex<mpsc::Receiver<StatusUpdate>>,
    pub bot_id: RwLock<Option<Snowflake>>,
    total_rx: Mutex<u64>,
    seq: Arc<RwLock<Option<usize>>>,
    session_id: Arc<RwLock<Option<String>>>,
    writer: RwLock<Option<mpsc::Sender<OutboundMessage>>>,
    kill_heartbeat: Mutex<Option<oneshot::Sender<()>>>,
    pub kill_shard_tx: mpsc::Sender<()>,
    kill_shard_rx: Mutex<mpsc::Receiver<()>>,
    last_ack: RwLock<Instant>,
    last_heartbeat: RwLock<Instant>,
    connect_time: RwLock<Instant>,
    ready_tx: Mutex<Option<mpsc::Sender<u16>>>,
    ready_guild_count: AtomicU16,
    received_count: AtomicU16,
}

// 16 KiB
const CHUNK_SIZE: usize = 16 * 1024;

impl Shard {
    pub fn new(
        identify: payloads::Identify,
        large_sharding_buckets: u16,
        cache: Arc<PostgresCache>,
        redis: Arc<Pool>,
        is_whitelabel: bool,
        error_tx: mpsc::Sender<FatalError>,
        ready_tx: Option<mpsc::Sender<u16>>,
    ) -> Arc<Shard> {
        let (kill_shard_tx, kill_shard_rx) = mpsc::channel(1);
        let (status_update_tx, status_update_rx) = mpsc::channel(1);

        let shard = Arc::new(Shard {
            identify,
            large_sharding_buckets,
            cache,
            redis,
            is_whitelabel,
            error_tx,
            status_update_tx,
            status_update_rx: Mutex::new(status_update_rx),
            bot_id: RwLock::new(None),
            total_rx: Mutex::new(0),
            seq: Arc::new(RwLock::new(None)),
            session_id: Arc::new(RwLock::new(None)),
            writer: RwLock::new(None),
            kill_heartbeat: Mutex::new(None),
            kill_shard_tx,
            kill_shard_rx: Mutex::new(kill_shard_rx),
            last_ack: RwLock::new(Instant::now()),
            last_heartbeat: RwLock::new(Instant::now()),
            connect_time: RwLock::new(Instant::now()), // will be overwritten
            ready_tx: Mutex::new(ready_tx),
            ready_guild_count: AtomicU16::new(0),
            received_count: AtomicU16::new(0),
        });

        shard
    }

    pub async fn connect(self: Arc<Self>) -> Result<(), GatewayError> {
        //rst
        *self.total_rx.lock().await = 0;
        self.ready_guild_count.store(0, Ordering::Relaxed);
        self.received_count.store(0, Ordering::Relaxed);
        // rst

        let uri = url::Url::parse("wss://gateway.discord.gg/?v=6&encoding=json&compress=zlib-stream").unwrap();

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
    pub async fn kill(self: Arc<Self>) {
        // BIG problem
        // TODO: Make this good
        tokio::spawn(async move {
            // TODO: panic?
            if let Err(e) = self.kill_shard_tx.clone().send(()).await {
                self.log_err("Failed to kill", &GatewayError::SendError(e));
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
                _ = kill_rx.recv() => {
                    self.log("Received kill message");
                    break;
                }

                // handle incoming payload
                payload = ws_rx.next() => {
                    match payload {
                        None => {
                            self.log("Payload was None, killing");
                            self.kill().await;
                            break;
                        }

                        Some(Err(e)) => {
                            self.log_err("Error reading data from websocket, killing", &GatewayError::WebsocketError(e));
                            self.kill().await;
                            break;
                        }

                        Some(Ok(Message::Close(frame))) => {
                            self.log(format!("Got close from gateway: {:?}", frame));
                            Arc::clone(&self).kill().await;

                            if let Some(frame) = frame {
                                if let CloseCode::Library(code) = frame.code {
                                    let fatal_codes: [u16; 2] = [4004, 4014];
                                    if fatal_codes.contains(&code) {
                                        if let Err(e) = self.error_tx.clone().send(FatalError::new(self.identify.data.token.clone(), frame.code, frame.reason.to_string())).await {
                                            self.log_err("Error pushing fatal error", &GatewayError::SendErrorError(e));
                                        }
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
                self.kill().await;
            }

            Opcode::InvalidSession => {
                self.log("Received invalid session payload from Discord");

                *self.session_id.write().await = None;
                *self.seq.write().await = None;

                self.kill().await;
            }

            Opcode::Hello => {
                let hello: payloads::Hello = serde_json::from_slice(&raw[..])?;
                let interval = Duration::from_millis(hello.data.heartbeat_interval as u64);

                let mut should_identify = true;
                if let (Some(session_id), Some(seq)) = (self.session_id.read().await.as_ref().cloned(), *self.seq.read().await) {
                    if let Err(e) = Arc::clone(&self).do_resume(session_id.clone(), seq).await {
                        self.log_err("Error RESUMEing, going to IDENTIFY", &e);

                        // rst
                        *self.session_id.write().await = None;
                        *self.seq.write().await = None;

                        self.wait_for_ratelimit().await?;

                        if self.connect_time.read().await.elapsed() > interval {
                            self.log("Connected over 45s ago, Discord will kick us off. Reconnecting.");
                            Arc::clone(&self).kill().await;
                            return Ok(());
                        } else {
                            should_identify = Arc::clone(&self).do_identify().await.is_err();
                        }
                    } else {
                        should_identify = false;
                        self.log("Resumed");
                    }
                }

                if should_identify {
                    self.wait_for_ratelimit().await?;

                    if self.connect_time.read().await.elapsed() > interval {
                        self.log("Connected over 45s ago, Discord will kick us off. Reconnecting.");
                        self.kill().await;
                        return Ok(());
                    }

                    if let Err(e) = Arc::clone(&self).do_identify().await {
                        self.log_err("Error identifying, killing", &e);
                        self.kill().await;
                        return e.into();
                    }

                    self.log("Identified");
                }

                let kill_tx = Arc::clone(&self).start_heartbeat(interval).await;
                *self.kill_heartbeat.lock().await = Some(kill_tx)
            }

            Opcode::HeartbeatAck => {
                *self.last_ack.write().await = Instant::now();
            }

            _ => {}
        }

        Ok(())
    }

    async fn handle_event(self: Arc<Self>, data: Box<RawValue>) -> Result<(), GatewayError> {
        //let payload: Dispatch = simd_json::from_str(&mut data.get()).map_err(GatewayError::SimdJsonError)?;
        let payload: Dispatch = serde_json::from_str( data.get()).map_err(GatewayError::JsonError)?;

        // Gateway events
        match &payload.data {
            Event::Ready(ready) => {
                *self.session_id.write().await = Some(ready.session_id.clone());
                *self.bot_id.write().await = Some(ready.user.id);
                self.ready_guild_count.store(ready.guilds.len() as u16, Ordering::Relaxed);

                self.log(format!("Ready on {}#{} ({})", ready.user.username, ready.user.discriminator, ready.user.id));
                return Ok(());
            }

            Event::Resumed(_) => {
                self.log(format!("Resumed on {:?}", *self.bot_id.read().await.unwrap_or(Snowflake(0))));
                return Ok(());
            }

            Event::GuildCreate(_) => {
                let received = self.received_count.fetch_add(1, Ordering::Relaxed);
                if received >= (self.ready_guild_count.load(Ordering::Relaxed) / 100) * 90 { // Once we have 90% of the guilds, we're ok to load more shards
                    if let Some(mut tx) = self.ready_tx.lock().await.take() {
                        if let Err(e) = tx.send(self.get_shard_id()).await.map_err(GatewayError::SendU16Error) {
                            self.log_err("Error sending ready notification to probe", &e);
                        }
                    }
                }
            }

            _ => {}
        }

        // cache + push to redis
        tokio::spawn(async move {
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

            // push to redis, even if error occurred
            // prepare redis payload
            if let Some(bot_id) = *self.bot_id.read().await {
                let wrapped = event_forwarding::Event {
                    bot_token: &self.identify.data.token[..],
                    bot_id: bot_id.0,
                    is_whitelabel: self.is_whitelabel,
                    shard_id: self.get_shard_id(),
                    event: &data
                };

                match serde_json::to_string(&wrapped).map_err(GatewayError::JsonError) {
                    Ok(json) => {
                        match self.redis.get().await.map_err(GatewayError::PoolError) {
                            Ok(mut conn) => {
                                let res = cmd("RPUSH")
                                    .arg(&[event_forwarding::KEY, &json[..]])
                                    .execute_async(&mut conn)
                                    .await
                                    .map_err(GatewayError::RedisError);

                                if let Err(e) = res {
                                    self.log_err("Error pushing event to Redis", &e);
                                }
                            },
                            Err(e) => self.log_err("Error pushing event to Redis", &e)
                        }
                    }
                    Err(e) => self.log_err("Error serializing redis payload", &e),
                }
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
                    shard.log("Hasn't received heartbeat, killing");
                    shard.kill().await;
                    break;
                }

                if let Err(e) = Arc::clone(&shard).do_heartbeat().await {
                    shard.log_err("Error sending heartbeat, killing", &e);
                    shard.kill().await;
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
        let key = match self.is_whitelabel {
            true => {
                let bot_id = match *self.bot_id.read().await {
                    Some(bot_id) => bot_id,
                    None => return GatewayError::MissingEventData.into(),
                };

                format!("ratelimiter:whitelabel:identify:{}", bot_id)
            }
            false => format!("ratelimiter:public:identify:{}", self.get_shard_id() % self.large_sharding_buckets),
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

    /// helper
    fn get_shard_id(&self) -> u16 {
        self.identify.data.shard_info.shard_id
    }

    pub fn log(&self, msg: impl Display) {
        println!("[{:0>2}] {}", self.get_shard_id(), msg);
    }

    pub fn log_err(&self, msg: impl Display, err: &GatewayError) {
        eprintln!("[{:0>2}] {}: {}", self.get_shard_id(), msg, err);
    }
}