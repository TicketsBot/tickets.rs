use std::fmt::Display;
use std::str;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use deadpool_redis::{cmd, Pool};
#[cfg(feature = "compression")]
use flate2::{Decompress, FlushDecompress, Status};
use futures::StreamExt;
use futures_util::SinkExt;
use log::{debug, error, info};
use serde::Serialize;
use serde_json::value::RawValue;
use tokio::sync::{mpsc, oneshot};
use parking_lot::{Mutex, RwLock};

use tokio::time::sleep;
use url::Url;

use cache::MemoryCache;
use common::event_forwarding;
#[cfg(feature = "whitelabel")]
use database::Database;
use model::guild::Member;
use model::user::StatusUpdate;
use model::Snowflake;

use crate::config::Config;
use crate::gateway::payloads::PresenceUpdate;
use crate::gateway::whitelabel_utils::is_whitelabel;
use crate::gateway::GatewayError;

use super::payloads;
use super::payloads::event::Event;
use super::payloads::{Dispatch, Opcode, Payload};
use super::OutboundMessage;
use crate::gateway::event_forwarding::{is_whitelisted, EventForwarder};
use serde_json::error::Category;
use serde_json::Value;
use std::error::Error;
use tokio_tungstenite::{
    connect_async, tungstenite,
    tungstenite::{protocol::frame::coding::CloseCode, Message},
};
use cache::model::CachedMember;
use std::ops::Deref;
use dashmap::DashMap;
use crate::GuildState;

const GATEWAY_VERSION: u8 = 9;

pub struct Shard<T: EventForwarder> {
    pub(crate) config: Arc<Config>,
    pub(crate) identify: payloads::Identify,
    large_sharding_buckets: u16,
    cache: Arc<MemoryCache>,
    redis: Arc<Pool>,
    pub status_update_tx: mpsc::Sender<StatusUpdate>,
    status_update_rx: Mutex<mpsc::Receiver<StatusUpdate>>,
    pub(crate) user_id: Snowflake,
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
    guild_states: DashMap<Snowflake, GuildState>,
    pub(crate) event_forwarder: Arc<T>,

    #[cfg(feature = "whitelabel")]
    pub(crate) database: Arc<Database>,
}

#[cfg(feature = "compression")]
const CHUNK_SIZE: usize = 16 * 1024; // 16KiB

impl<T: EventForwarder> Shard<T> {
    pub fn new(
        config: Arc<Config>,
        identify: payloads::Identify,
        large_sharding_buckets: u16,
        cache: Arc<MemoryCache>,
        redis: Arc<Pool>,
        user_id: Snowflake,
        event_forwarder: Arc<T>,
        #[cfg(feature = "whitelabel")] database: Arc<Database>,
    ) -> Arc<Shard<T>> {
        let (kill_shard_tx, kill_shard_rx) = oneshot::channel();
        let (status_update_tx, status_update_rx) = mpsc::channel(1);

        Arc::new(Shard {
            config,
            identify,
            large_sharding_buckets,
            cache,
            redis,
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
            guild_states: DashMap::new(),
            event_forwarder,
            #[cfg(feature = "whitelabel")]
            database,
        })
    }

    pub async fn connect(self: Arc<Self>) -> Result<(), GatewayError> {
        // rst
        let (kill_shard_tx, kill_shard_rx) = oneshot::channel();
        *self.kill_shard_tx.lock() = Some(kill_shard_tx);
        *self.kill_shard_rx.lock() = kill_shard_rx;

        *self.total_rx.lock() = 0;

        *self.last_heartbeat.write() = Instant::now();
        *self.last_ack.write() = Instant::now();
        // rst

        let mut uri = format!(
            "wss://gateway.discord.gg/?v={}&encoding=json",
            GATEWAY_VERSION
        );
        if cfg!(feature = "compression") {
            uri.push_str("&compress=zlib-stream");
        }

        let uri = Url::parse(&uri[..]).expect("Failed to parse websocket uri");

        self.wait_for_ratelimit().await;

        let (wss, _) = connect_async(uri).await?;
        let (ws_tx, ws_rx) = wss.split();
        *self.connect_time.write() = Instant::now();

        // start writer
        let (recv_broker_tx, recv_broker_rx) = futures::channel::mpsc::unbounded();
        let (send_broker_tx, send_broker_rx) = futures::channel::mpsc::unbounded();
        let (internal_tx, internal_rx) = mpsc::channel(1);
        tokio::spawn(handle_writes(send_broker_tx, internal_rx));

        let forward_outbound = send_broker_rx.map(Ok).forward(ws_tx);
        let forward_inbound = ws_rx.map(Ok).forward(recv_broker_tx);

        *self.writer.write() = Some(internal_tx);

        tokio::spawn(async move {
            futures::future::select(forward_outbound, forward_inbound).await;
        });

        // start read loop
        if let Err(e) = self.listen(recv_broker_rx).await {
            return Err(e);
        }

        Ok(())
    }

    // helper function
    async fn write<U: Serialize>(
        &self,
        msg: U,
        tx: oneshot::Sender<Result<(), futures::channel::mpsc::SendError>>,
    ) -> Result<(), GatewayError> {
        OutboundMessage::new(msg, tx)?
            .send(self.writer.read().clone().unwrap())
            .await?;

        Ok(())
    }

    // helper function
    pub fn kill(self: Arc<Self>) {
        // BIG problem
        // TODO: Make this good
        tokio::spawn(async move {
            // TODO: panic?
            let kill_shard_tx = self.kill_shard_tx.lock().take();
            let kill_heartbeat_tx = self.kill_heartbeat.lock().take();

            match kill_shard_tx {
                Some(kill_shard_tx) => {
                    if kill_shard_tx.send(()).is_err() {
                        self.log_err(
                            "Failed to kill",
                            &GatewayError::custom("Receiver already unallocated"),
                        );
                    }
                }
                None => self.log("Tried to kill but kill_shard_tx was None"),
            }

            match kill_heartbeat_tx {
                Some(kill_heartbeat_tx) => {
                    if kill_heartbeat_tx.send(()).is_err() {
                        self.log_err(
                            "Failed to kill heartbeat",
                            &GatewayError::custom("Receiver already unallocated"),
                        );
                    }
                }
                None => self.log("Tried to kill but kill_heartbeat_tx was None"),
            }
        });
    }

    async fn listen(
        self: Arc<Self>,
        mut rx: futures::channel::mpsc::UnboundedReceiver<
            Result<Message, tokio_tungstenite::tungstenite::Error>,
        >,
    ) -> Result<(), GatewayError> {
        #[cfg(feature = "compression")]
            let mut decoder = Decompress::new(true);

        loop {
            let shard = Arc::clone(&self);
            let kill_rx = &mut *shard.kill_shard_rx.lock();
            let status_update_rx = &mut shard.status_update_rx.lock();

            tokio::select! {
                // handle kill
                _ = kill_rx => {
                    self.log("Received kill message");
                    break;
                }

                // handle incoming payload
                payload = rx.next() => {
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

                        Some(Ok(Message::Text(data))) => {
                            let value: Value = serde_json::from_slice(data.as_bytes())?;

                            let payload = match Arc::clone(&self).read_payload(&value).await {
                                Ok(payload) => payload,
                                Err(e) => {
                                    self.log_err("Error while deserializing payload", &e);
                                    continue;
                                }
                            };

                            if let Err(e) = Arc::clone(&self).process_payload(payload, value).await {
                                self.log_err("An error occurred while processing a payload", &e);
                            }
                        }

                        #[cfg(feature = "compression")]
                        Some(Ok(Message::Binary(data))) => {
                            let data = match Arc::clone(&self).decompress(data, &mut decoder).await {
                                Ok(data) => data,
                                Err(e) => {
                                    self.log_err("Error while decompressing payload", &e);
                                    continue;
                                }
                            };

                            let value: Value = serde_json::from_slice(data.as_bytes())?;

                            let payload = match Arc::clone(&self).read_payload(&value).await {
                                Ok(payload) => payload,
                                Err(e) => {
                                    self.log_err("Error while deserializing payload", &e);
                                    continue;
                                }
                            };

                            if let Err(e) = Arc::clone(&self).process_payload(payload, value).await {
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
                                Ok(Err(e)) => shard.log_err("Error writing presence update payload", &GatewayError::WebsocketSendError(e)),
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

    #[cfg(feature = "compression")]
    async fn decompress(
        self: Arc<Self>,
        data: Vec<u8>,
        decoder: &mut Decompress,
    ) -> Result<Vec<u8>, GatewayError> {
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

    // Manually deserialize since we only need 2 values
    async fn read_payload(self: Arc<Self>, data: &Value) -> Result<Payload, GatewayError> {
        let opcode = serde_json::from_value(
            data.get("op")
                .ok_or_else(|| GatewayError::MissingFieldError("op".to_owned()))?
                .clone(),
        )?;

        let seq = match data.get("s") {
            None => None,
            Some(s) => serde_json::from_value(s.clone())?,
        };

        Ok(Payload { opcode, seq })
    }

    async fn process_payload(
        self: Arc<Self>,
        payload: Payload,
        raw: Value,
    ) -> Result<(), GatewayError> {
        if let Some(seq) = payload.seq {
            *self.seq.write() = Some(seq);
        }

        match payload.opcode {
            Opcode::Dispatch => {
                let payload = serde_json::from_value(raw)?;

                if let Err(e) = Arc::clone(&self).handle_event(payload).await {
                    if let GatewayError::JsonError(ref err) = e {
                        // Ignore unknown payloads
                        if err.classify() != Category::Data {
                            self.log_err("Error processing dispatch", &e);
                        } else {
                            self.log_err("Error processing dispatch", &e);
                        }
                    } else {
                        self.log_err("Error processing dispatch", &e);
                    }
                }
            }

            Opcode::Reconnect => {
                self.log("Received reconnect payload from Discord");
                self.kill();
            }

            Opcode::InvalidSession => {
                self.log("Received invalid session payload from Discord");

                *self.session_id.write() = None;
                *self.seq.write() = None;

                self.kill();
            }

            Opcode::Hello => {
                let hello: payloads::Hello = serde_json::from_value(raw)?;
                let interval = Duration::from_millis(hello.data.heartbeat_interval as u64);

                let mut should_identify = true;

                let session_id = self.session_id.read().as_ref().cloned();
                let seq = *self.seq.read();
                if let (Some(session_id), Some(seq)) = (session_id, seq) {
                    if let Err(e) = Arc::clone(&self).do_resume(session_id.clone(), seq).await {
                        self.log_err("Error RESUMEing, going to IDENTIFY", &e);

                        // rst
                        *self.session_id.write() = None;
                        *self.seq.write() = None;

                        //self.wait_for_ratelimit().await?;

                        if self.connect_time.read().elapsed() > interval {
                            self.log(
                                "Connected over 45s ago, Discord will kick us off. Reconnecting.",
                            );
                            Arc::clone(&self).kill();
                            return Ok(());
                        }
                    } else {
                        should_identify = false;
                        self.log("Sent resume successfully");
                    }
                }

                if should_identify {
                    //self.wait_for_ratelimit().await?;
                    if self.connect_time.read().elapsed() > interval {
                        self.log("Connected over 45s ago, Discord will kick us off. Reconnecting.");
                        self.kill();
                        return Ok(());
                    }

                    if let Err(e) = Arc::clone(&self).do_identify().await {
                        let _ = self.reset_ratelimit().await;
                        self.log_err("Error identifying, killing", &e);
                        self.kill();
                        return e.into();
                    }

                    if let Err(e) = self.reset_ratelimit().await {
                        self.log_err("Error resetting ratelimit: {}", &e);
                    }

                    self.log("Identified");
                }

                let kill_tx = Arc::clone(&self).start_heartbeat(interval).await;
                *self.kill_heartbeat.lock() = Some(kill_tx)
            }

            Opcode::HeartbeatAck => {
                *self.last_ack.write() = Instant::now();
            }

            _ => {}
        }

        Ok(())
    }

    async fn handle_event(self: Arc<Self>, data: Box<RawValue>) -> Result<(), GatewayError> {
        let payload: Dispatch = serde_json::from_str(data.get())?;

        // Gateway events
        match &payload.data {
            Event::Ready(ready) => {
                *self.session_id.write() = Some(ready.session_id.clone());

                self.log(format!(
                    "Ready on {}#{} ({})",
                    ready.user.username, ready.user.discriminator, ready.user.id
                ));
                return Ok(());
            }

            Event::Resumed(_) => {
                self.log("Received resumed acknowledgement");
                return Ok(());
            }

            #[cfg(feature = "whitelabel")]
            Event::GuildCreate(g) => {
                #[cfg(feature = "whitelabel")]
                if let Err(e) = self.store_whitelabel_guild(g.id).await {
                    self.log_err("Error while storing whitelabel guild data", &e);
                }
            }

            _ => {}
        }

        // cache + push to redis
        let should_forward =
            is_whitelisted(&payload.data) && self.meets_forward_threshold(&payload.data).await && false;

        // cache
        let res = match payload.data {
            Event::ChannelCreate(channel) => self.cache.store_channel(channel),
            Event::ChannelUpdate(channel) => self.cache.store_channel(channel),
            Event::ChannelDelete(channel) => self.cache.delete_channel(channel.id, channel.guild_id),
            Event::ThreadCreate(thread) => self.cache.store_channel(thread),
            Event::ThreadUpdate(thread) => self.cache.store_channel(thread),
            Event::ThreadDelete(thread) => self.cache.delete_channel(thread.id, Some(thread.guild_id)),
            Event::GuildCreate(guild) => {
                let count = self.cache.get_guild_count().unwrap_or(0);
                if count % 1000 == 0 {
                    println!("Guilds: {:?}", self.cache.get_guild_count());
                }

                let guild_id = guild.id;
                self.cache.store_guild(guild);
                self.guild_states.insert(guild_id, GuildState::Ready);
                Ok(())
            }
            Event::GuildUpdate(data) => {
                match self.cache.guild_mut(data.id) {
                    Some(mut guild) => {
                        data.apply_changes(&mut guild.value_mut().guild);
                    }
                    None => self.log("Got guild update for unknown guild"),
                };

                Ok(())
            }
            Event::GuildDelete(guild) => {
                if guild.unavailable.is_none() {
                    // we were kicked
                    self.cache.delete_guild(guild.id)
                } else {
                    Ok(())
                }
            }
            Event::GuildBanAdd(ev) => self.cache.delete_member(ev.user.id, ev.guild_id),
            Event::GuildEmojisUpdate(ev) => {
                self.cache.store_emojis(ev.emojis, ev.guild_id)
            }
            Event::GuildMemberAdd(ev) => self.cache.store_member(ev.member, ev.guild_id),
            Event::GuildMemberRemove(ev) => {
                self.cache.delete_member(ev.user.id, ev.guild_id)
            }
            Event::GuildMemberUpdate(ev) => {
                self.cache.store_user(ev.user.clone());

                match self.cache.guild_mut(ev.guild_id) {
                    Some(mut guild) => {
                        let member = guild.value_mut().members.get_mut(&ev.user.id);
                        if let Some(mut member) = member {
                            ev.apply_changes(member.value_mut());
                        } else {
                            drop(member); // Verbosely prevent deadlock

                            let user_id = ev.user.id;
                            let member: Member = ev.into();
                            guild.members.insert(user_id, CachedMember::from(member)); // TODO: Potential race condition
                        }
                    }

                    None => self.log("Got guild member update for unknown guild"),
                };

                Ok(())
            }
            Event::GuildMembersChunk(ev) => {
                self.cache.store_members(ev.members, ev.guild_id)
            }
            Event::GuildRoleCreate(ev) => self.cache.store_role(ev.role, ev.guild_id),
            Event::GuildRoleUpdate(ev) => self.cache.store_role(ev.role, ev.guild_id),
            Event::GuildRoleDelete(ev) => self.cache.delete_role(ev.role_id, ev.guild_id),
            Event::UserUpdate(user) => self.cache.store_user(user),
            _ => Ok(()),
        };

        if let Err(e) = res {
            self.log_err("Error updating cache", &GatewayError::CacheError(e));
        }

        // push to workers, even if error occurred
        if should_forward {
            tokio::spawn(async move {
                // prepare payload
                let wrapped = event_forwarding::Event {
                    bot_token: &self.identify.data.token[..],
                    bot_id: self.user_id.0,
                    is_whitelabel: is_whitelabel(),
                    shard_id: self.get_shard_id(),
                    event: &data,
                };

                if let Err(e) = self
                    .event_forwarder
                    .forward_event(self.config.clone().deref(), wrapped)
                    .await
                {
                    self.log_err("Error while executing worker HTTP request", &e);
                }
            });
        }

        Ok(())
    }

    async fn meets_forward_threshold(&self, event: &Event) -> bool {
        true
    }

    // returns cancellation channel
    async fn start_heartbeat(self: Arc<Self>, interval: Duration) -> oneshot::Sender<()> {
        let (cancel_tx, mut cancel_rx) = oneshot::channel();

        tokio::spawn(async move {
            sleep(interval).await;

            let mut has_done_heartbeat = false;
            while let Err(oneshot::error::TryRecvError::Empty) = cancel_rx.try_recv() {
                let shard = Arc::clone(&self);

                // if done inline, clippy complains about evaluation order
                let last_ack = shard.last_ack.read();
                let last_heartbeat = shard.last_heartbeat.read();
                let elapsed = last_ack.checked_duration_since(*last_heartbeat);
                drop(last_heartbeat); // drop here so that a reference is no longer held to shard
                drop(last_ack);

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

                sleep(interval).await;
            }
        });

        cancel_tx
    }

    async fn do_heartbeat(self: Arc<Self>) -> Result<(), GatewayError> {
        let payload = payloads::Heartbeat::new(*self.seq.read());

        let (tx, rx) = oneshot::channel();
        self.write(payload, tx).await?;

        rx.await??;

        *self.last_heartbeat.write() = Instant::now();
        Ok(())
    }

    async fn do_identify(self: Arc<Self>) -> Result<(), GatewayError> {
        let (tx, rx) = oneshot::channel();
        self.write(&self.identify, tx).await?;

        Ok(rx.await??)
    }

    async fn wait_for_ratelimit(&self) -> Result<(), GatewayError> {
        let key = if is_whitelabel() {
            format!("ratelimiter:whitelabel:identify:{}", self.user_id)
        } else {
            format!(
                "ratelimiter:public:identify:{}",
                self.get_shard_id() % self.large_sharding_buckets
            )
        };

        let mut res = redis::Value::Nil;
        while res == redis::Value::Nil {
            let mut conn = self.redis.get().await?;

            res = cmd("SET")
                .arg(&[&key[..], "1", "NX", "PX", "6000"]) // some arbitrary value, set if not exist, set expiry, of 6s
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

        Ok(())
    }

    async fn reset_ratelimit(&self) -> Result<(), GatewayError> {
        let key = if is_whitelabel() {
            format!("ratelimiter:whitelabel:identify:{}", self.user_id)
        } else {
            format!(
                "ratelimiter:public:identify:{}",
                self.get_shard_id() % self.large_sharding_buckets
            )
        };

        let mut conn = self.redis.get().await?;

        let res = cmd("SET")
            .arg(&[&key[..], "1", "PX", "6000"]) // some arbitrary value, set expiry, of 6s
            .query_async(&mut conn)
            .await?;

        Ok(())
    }

    /// Shard.session_id & Shard.seq should not be None when calling this function
    /// if they are, the function will panic
    async fn do_resume(
        self: Arc<Self>,
        session_id: String,
        seq: usize,
    ) -> Result<(), GatewayError> {
        let payload = payloads::Resume::new(self.identify.data.token.clone(), session_id, seq);

        let (tx, rx) = oneshot::channel();
        self.write(payload, tx).await?;

        Ok(rx.await??)
    }

    pub fn get_guild_state(&self, guild_id: Snowflake) -> GuildState {
        self.guild_states.get(&guild_id).map(|state| *state.value()).unwrap_or(GuildState::Waiting)
    }

    /// helper
    pub fn get_shard_id(&self) -> u16 {
        self.identify.data.shard_info.shard_id
    }

    pub fn log(&self, msg: impl Display) {
        if is_whitelabel() {
            info!("[shard:{}] {}", self.user_id, msg);
        } else {
            info!("[shard:{:0>2}] {}", self.get_shard_id(), msg);
        }
    }

    pub fn log_err(&self, msg: impl Display, err: &GatewayError) {
        if is_whitelabel() {
            error!("[shard:{}] {}: {}", self.user_id, msg, err);
        } else {
            error!("[shard:{:0>2}] {}: {}", self.get_shard_id(), msg, err);
        }
    }

    pub fn log_debug(&self, msg: impl Display, raw_payload: &str, err: impl Error) {
        if is_whitelabel() {
            debug!(
                "[shard:{}] {}: {}\nFull payload: {}",
                self.user_id, msg, err, raw_payload
            );
        } else {
            debug!(
                "[shard:{:0>2}] {}: {}\nFull payload: {}",
                self.get_shard_id(),
                msg,
                err,
                raw_payload
            );
        }
    }
}

async fn handle_writes(
    mut tx: futures::channel::mpsc::UnboundedSender<tungstenite::Message>,
    mut rx: mpsc::Receiver<super::OutboundMessage>,
) {
    while let Some(msg) = rx.recv().await {
        let payload = Message::text(msg.message);
        let res = tx.send(payload).await;

        if let Err(e) = msg.tx.send(res) {
            eprintln!("Error while sending write result back to caller: {:?}", e);
        }
    }
}

#[cfg(feature = "postgres-cache")]
fn apply_guild_id_to_channels(guild: &mut Guild) {
    for channel in &mut guild.channels {
        channel.guild_id = Some(guild.id)
    }

    for thread in &mut guild.threads {
        thread.guild_id = Some(guild.id)
    }
}
