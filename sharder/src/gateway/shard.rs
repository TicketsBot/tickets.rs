use super::payloads;
use super::payloads::{Opcode, Payload};
use super::payloads::event::Event;

use super::OutboundMessage;

use std::error::Error;

use flate2::{Decompress, FlushDecompress, Status};

use tokio::time::delay_for;
use std::time::Duration;
use std::time::Instant;

use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};

use tokio::sync::{mpsc, oneshot};
use futures_util::{StreamExt, SinkExt, stream::{SplitSink, SplitStream}};
use tokio_tungstenite::{WebSocketStream, connect_async, tungstenite::Message};
use tokio::net::TcpStream;
use tokio_tls::TlsStream;
use serde::Serialize;
use cache::{Cache, PostgresCache};
use model::guild::Member;
use chrono::Utc;
use std::ops::Sub;
use model::channel::Channel;
use model::Snowflake;
use common::event_forwarding;
use crate::gateway::payloads::{Dispatch, PresenceUpdate};
use serde_json::{Value, Map};
use crate::gateway::GatewayError;
use crate::manager::FatalError;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use model::user::StatusUpdate;
use darkredis::{ConnectionPool, Command};

pub struct Shard {
    identify: payloads::Identify,
    large_sharding_buckets: u16,
    cache: Arc<PostgresCache>,
    redis: Arc<ConnectionPool>,
    is_whitelabel: bool,
    error_tx: Mutex<mpsc::Sender<FatalError>>,
    pub status_update_tx: mpsc::Sender<StatusUpdate>,
    bot_id: Arc<RwLock<Option<Snowflake>>>,
    total_rx: Mutex<u64>,
    seq: Arc<RwLock<Option<usize>>>,
    session_id: Arc<RwLock<Option<String>>>,
    writer: RwLock<Option<mpsc::Sender<OutboundMessage>>>,
    kill_heartbeat: Mutex<Option<oneshot::Sender<()>>>,
    pub kill_shard_tx: mpsc::Sender<()>,
    kill_shard_rx: Mutex<mpsc::Receiver<()>>,
    last_ack: RwLock<Instant>,
    last_heartbeat: RwLock<Instant>,
}

// 16 KiB
const CHUNK_SIZE: usize = 16 * 1024;

type WebSocketTx = SplitSink<WebSocketStream<tokio_tungstenite::stream::Stream<TcpStream, TlsStream<TcpStream>>>, Message>;
type WebSocketRx = SplitStream<WebSocketStream<tokio_tungstenite::stream::Stream<TcpStream, TlsStream<TcpStream>>>>;

impl Shard {
    pub fn new(
        identify: payloads::Identify,
        large_sharding_buckets: u16,
        cache: Arc<PostgresCache>,
        redis: Arc<ConnectionPool>,
        is_whitelabel: bool,
        error_tx: mpsc::Sender<FatalError>,
    ) -> Arc<Shard> {
        let (kill_shard_tx, kill_shard_rx) = mpsc::channel(1);
        let (status_update_tx, status_update_rx) = mpsc::channel(1);

        let shard = Arc::new(Shard {
            identify,
            large_sharding_buckets,
            cache,
            redis,
            is_whitelabel,
            error_tx: Mutex::new(error_tx),
            status_update_tx,
            bot_id: Arc::new(RwLock::new(None)),
            total_rx: Mutex::new(0),
            seq: Arc::new(RwLock::new(None)),
            session_id: Arc::new(RwLock::new(None)),
            writer: RwLock::new(None),
            kill_heartbeat: Mutex::new(None),
            kill_shard_tx,
            kill_shard_rx: Mutex::new(kill_shard_rx),
            last_ack: RwLock::new(Instant::now()),
            last_heartbeat: RwLock::new(Instant::now()),
        });

        Arc::clone(&shard).listen_status_updates(status_update_rx);

        shard
    }

    pub async fn connect(self: Arc<Self>) -> Result<(), Box<dyn Error>> {
        *self.total_rx.lock().await = 0; // rst

        let uri = url::Url::parse("wss://gateway.discord.gg/?v=6&encoding=json&compress=zlib-stream").unwrap();

        let (wss, _) = connect_async(uri).await?;
        let (ws_tx, ws_rx) = wss.split();

        // start writer
        let (writer_tx, writer_rx) = mpsc::channel(16);
        tokio::spawn(async move {
            Shard::handle_writes(ws_tx, writer_rx).await;
        });
        *self.writer.write().await = Some(writer_tx);

        // start read loop
        if let Err(e) = self.listen(ws_rx).await {
            return Err(Box::new(e));
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
    async fn kill(self: Arc<Self>) {
        // BIG problem
        // TODO: panic?
        if let Err(e) = self.kill_shard_tx.clone().send(()).await {
            eprintln!("Failed to kill shard! {}", e);
        }
    }

    fn listen_status_updates(self: Arc<Self>, mut status_update_rx: mpsc::Receiver<StatusUpdate>) {
        tokio::spawn(async move {
            while let Some(msg) = status_update_rx.recv().await {
                let shard = Arc::clone(&self);

                // create payload
                let payload = PresenceUpdate::new(msg);

                let (res_tx, res_rx) = oneshot::channel();

                match OutboundMessage::new(payload, res_tx) {
                    Ok(msg) => {
                        if let Some(writer) = &*shard.writer.read().await {
                            match msg.send(writer.clone()).await {
                                Ok(_) => {
                                    match res_rx.await {
                                        Ok(Err(e)) => eprintln!("Error writing status update: {}", e),
                                        Err(e) => eprintln!("Error writing status update: {}", e),
                                        _ => {}
                                    }
                                }
                                Err(e) => eprintln!("Error writing status update: {}", e),
                            }
                        }
                    }
                    Err(e) => eprintln!("Failed to serialize status update: {}", e),
                }
            }
        });
    }

    async fn listen(self: Arc<Self>, mut ws_rx: WebSocketRx) -> Result<(), GatewayError> {
        let mut decoder = Decompress::new(true);

        let shard = Arc::clone(&self);
        loop {
            let shard = Arc::clone(&shard);

            match shard.kill_shard_rx.lock().await.try_recv() {
                Err(mpsc::error::TryRecvError::Empty) => {}
                _ => {
                    // kill heartbeat loop
                    if let Some(kill_heartbeat) = shard.kill_heartbeat.lock().await.take() {
                        if let Err(_) = kill_heartbeat.send(()) {
                            eprintln!("Error killing heartbeat on shard {}", shard.get_shard_id());
                        }
                    }
                    break;
                }
            };

            match ws_rx.next().await {
                None => {
                    println!("Shard {} closed (next is None)", self.get_shard_id());
                    break;
                }

                Some(Err(e)) => {
                    return GatewayError::WebsocketError(e).into();
                }

                Some(Ok(msg)) => {
                    if let Message::Close(frame) = msg {
                        let frame = frame.unwrap_or(CloseFrame {
                            code: CloseCode::Invalid,
                            reason: Default::default(),
                        });

                        let wrapped = FatalError::new(self.identify.data.token.clone(), frame.code, frame.reason.to_string());

                        if let Err(e) = self.error_tx.lock().await.send(wrapped).await {
                            return GatewayError::SendErrorError(e).into();
                        }

                        return Ok(());
                    }

                    if !msg.is_binary() {
                        eprintln!("Received non-binary message");
                        continue;
                    }

                    let (payload, raw) = match shard.read_payload(msg, &mut decoder).await {
                        Some(r) => r,
                        _ => continue,
                    };

                    if let Err(e) = Arc::clone(&self).process_payload(payload, raw).await {
                        eprintln!("Error reading payload: {}", e);
                    }
                }
            };
        }

        Ok(())
    }

    // we return None because it's ok to discard the payload
    async fn read_payload(self: Arc<Self>, msg: tokio_tungstenite::tungstenite::protocol::Message, decoder: &mut Decompress) -> Option<(Payload, Vec<u8>)> {
        let compressed = msg.into_data();

        let mut total_rx = self.total_rx.lock().await;

        let mut output: Vec<u8> = Vec::with_capacity(CHUNK_SIZE);
        let before = total_rx.clone();
        let mut offset: usize = 0;

        while ((decoder.total_in() - *total_rx) as usize) < compressed.len() {
            let mut temp: Vec<u8> = Vec::with_capacity(CHUNK_SIZE);

            match decoder.decompress_vec(&compressed[offset..], &mut temp, FlushDecompress::Sync) {
                Ok(Status::StreamEnd) => break,
                Ok(Status::Ok) | Ok(Status::BufError) => {
                    output.append(&mut temp);
                    offset = (decoder.total_in() - before) as usize;
                }

                // TODO: Should we reconnect?
                Err(e) => {
                    eprintln!("Error while decompressing: {}", e);

                    *total_rx = 0;
                    decoder.reset(true);

                    return None;
                }
            };
        }

        *total_rx = decoder.total_in();

        // deserialize payload
        match serde_json::from_slice(&output[..]) {
            Ok(payload) => Some((payload, output)),
            Err(e) => {
                eprintln!("Error occurred while deserializing payload: {}", e);
                None
            }
        }
    }

    async fn process_payload(self: Arc<Self>, payload: Payload, raw: Vec<u8>) -> Result<(), GatewayError> {
        // update sequence number
        if let Some(seq) = payload.seq {
            let mut stored_seq = self.seq.write().await;
            *stored_seq = Some(seq);
        }

        // figure out which payload we need
        match payload.opcode {
            Opcode::Dispatch => {
                //let payload: payloads::Dispatch = serde_json::from_slice(&raw[..])?;
                // perform all deserialization in handle_event
                let payload: Map<String, Value> = serde_json::from_slice(&raw[..]).map_err(GatewayError::JsonError)?;
                self.handle_event(payload).await?;
                Ok(())
            }
            Opcode::Reconnect => {
                self.kill().await;
                Ok(())
            }
            Opcode::InvalidSession => {
                let res: Result<payloads::InvalidSession, serde_json::Error> = serde_json::from_slice(&raw[..]);

                println!("Shard {} got invalid session", self.get_shard_id());

                match &res {
                    Ok(payload) => {
                        if !payload.is_resumable {
                            *self.session_id.write().await = None;
                        }
                    }
                    Err(_) => {
                        *self.session_id.write().await = None;
                    }
                };

                self.kill().await;
                res.map(|_| ()).map_err(GatewayError::JsonError)
            }
            Opcode::Hello => {
                let hello: payloads::Hello = serde_json::from_slice(&raw[..])?;

                let res = if self.session_id.read().await.is_some() && self.seq.read().await.is_some() {
                    Arc::clone(&self).do_resume().await
                } else {
                    Arc::clone(&self).do_identify().await
                };

                // TODO: Fatal error log, check if we should remove token
                if let Err(e) = res {
                    eprintln!("Received error while authenticating: {}", e);
                    self.kill().await;
                    return Err(e);
                }

                // we unwrap here because writer should never be None.
                let kill_tx = Arc::clone(&self).start_heartbeat(hello.data.heartbeat_interval).await;

                *self.kill_heartbeat.lock().await = Some(kill_tx);

                Ok(())
            }
            Opcode::HeartbeatAck => {
                let mut last_ack = self.last_ack.write().await;
                *last_ack = Instant::now();

                Ok(())
            }
            _ => Ok(()),
        }
    }

    async fn handle_event(self: Arc<Self>, data: Map<String, Value>) -> Result<(), GatewayError> {
        let event_type = if let Some(v) = data.get("t") {
            if let Some(s) = v.as_str() {
                s
            } else {
                return Err(GatewayError::TypeNotString(v.clone().clone()));
            }
        } else {
            return Err(GatewayError::TypeNotString(Value::String("missing!".to_owned())));
        }.to_owned();

        let payload: Dispatch = serde_json::from_value(serde_json::Value::Object(data)).map_err(GatewayError::custom)?;
        let event: Event = serde_json::from_value(payload.data.clone()).map_err(GatewayError::custom)?;

        let mut extra = event_forwarding::Extra {
            is_join: false,
        };

        // Log ready
        if let Event::Ready(ready) = &event {
            *self.session_id.write().await = Some(ready.session_id.clone());
            *self.bot_id.write().await = Some(ready.user.id);
            println!("Connected on {}#{} ({}) shard {} / {}", ready.user.username, ready.user.discriminator, ready.user.id, ready.shard.shard_id, ready.shard.num_shards);
        }

        match &event {
            Event::GuildCreate(guild) => {
                let mut is_join = false;

                match self.cache.get_guild(guild.id).await {
                    Ok(Some(_)) => (),

                    Ok(None) => {
                        // don't mass DM everyone on cache purge
                        // check if the bot joined in the last minute
                        if let Some(joined_at) = guild.joined_at {
                            is_join = Utc::now().sub(joined_at).num_seconds() <= 60;
                        }
                    }

                    Err(e) => {
                        eprintln!("Error retrieving cached guild: {:?}", e);
                    }
                };

                extra.is_join = is_join;
            }
            _ => (),
        }

        // Update cache
        let cache = Arc::clone(&self.cache);
        tokio::spawn(async move {
            let res = match event {
                Event::ChannelCreate(channel) => cache.store_channel(&channel).await,
                Event::ChannelUpdate(channel) => cache.store_channel(&channel).await,
                Event::ChannelDelete(channel) => cache.delete_channel(channel.id).await,
                Event::GuildCreate(mut guild) => {
                    guild.channels = Shard::apply_guild_id_to_channels(guild.channels, guild.id);
                    cache.store_guild(&guild).await
                }
                Event::GuildUpdate(mut guild) => {
                    guild.channels = Shard::apply_guild_id_to_channels(guild.channels, guild.id);
                    cache.store_guild(&guild).await
                }
                Event::GuildDelete(guild) => cache.delete_guild(guild.id).await, // TODO: Check if just available or actual kick
                Event::GuildBanAdd(ev) => cache.delete_member(ev.user.id, ev.guild_id).await,
                Event::GuildEmojisUpdate(ev) => cache.store_emojis(ev.emojis.iter().collect(), ev.guild_id).await,
                Event::GuildMemberAdd(ev) => cache.store_member(&ev.member, ev.guild_id).await,
                Event::GuildMemberRemove(ev) => cache.delete_member(ev.user.id, ev.guild_id).await,
                Event::GuildMemberUpdate(ev) => cache.store_member(&Member {
                    user: Some(ev.user),
                    nick: ev.nick,
                    roles: ev.roles,
                    joined_at: ev.joined_at,
                    premium_since: ev.premium_since,
                    deaf: false, // TODO: Don't update these fields somehow?
                    mute: false, // TODO: Don't update these fields somehow?
                }, ev.guild_id).await,
                Event::GuildMembersChunk(ev) => cache.store_members(ev.members.iter().collect(), ev.guild_id).await,
                Event::GuildRoleCreate(ev) => cache.store_role(&ev.role, ev.guild_id).await,
                Event::GuildRoleUpdate(ev) => cache.store_role(&ev.role, ev.guild_id).await,
                Event::GuildRoleDelete(ev) => cache.delete_role(ev.role_id).await,
                Event::UserUpdate(user) => cache.store_user(&user).await,
                _ => Ok(()),
            };

            if let Err(e) = res {
                eprintln!("Error while updating cache: {:?}", e);
            }
        });

        let bot_id: Snowflake;

        {
            let guard = self.bot_id.read().await;
            if guard.is_none() {
                eprintln!("Bot is whitelabel but has no ID!");

                if self.is_whitelabel {
                    return Err(GatewayError::NoneId);
                } else {
                    // we don't need the ID... I think?
                    bot_id = Snowflake(0);
                }
            } else {
                bot_id = guard.unwrap();
            }
        }

        // push to redis
        let wrapped = event_forwarding::Event {
            bot_token: self.identify.data.token.clone(),
            bot_id: bot_id.0,
            is_whitelabel: self.is_whitelabel,
            shard_id: self.get_shard_id(),
            event_type: event_type.to_owned(),
            data: payload.data.get("d").unwrap(),
            extra,
        };

        let json = &serde_json::to_string(&wrapped).map_err(GatewayError::JsonError)?;

        self.redis.get().await
            .rpush(event_forwarding::KEY, json)
            .await
            .map_err(GatewayError::RedisError)?;

        Ok(())
    }

    fn apply_guild_id_to_channels(channels: Option<Vec<Channel>>, guild_id: Snowflake) -> Option<Vec<Channel>> {
        channels.map(|channels| channels.into_iter().map(|mut c| {
            c.guild_id = Some(guild_id);
            c
        }).collect())
    }

    // returns cancellation channel
    async fn start_heartbeat(
        self: Arc<Self>,
        interval: u32,
    ) -> oneshot::Sender<()> {
        let interval = Duration::from_millis(interval as u64);

        let (cancel_tx, mut cancel_rx) = oneshot::channel();

        tokio::spawn(async move {
            delay_for(interval).await;

            let mut has_sent_heartbeat = false;
            while let Err(oneshot::error::TryRecvError::Empty) = cancel_rx.try_recv() {
                // check if we've received an ack
                {
                    let last_ack = *self.last_ack.read().await;
                    let last_heartbeat = *self.last_heartbeat.read().await;
                    if has_sent_heartbeat && (last_heartbeat > last_ack || last_ack.duration_since(last_heartbeat) > interval) {
                        // BIG problem
                        // TODO: panic?
                        if let Err(e) = self.kill_shard_tx.clone().send(()).await {
                            eprintln!("Failed to kill shard! {}", e);
                        }

                        break;
                    }
                }

                let mut should_kill = false;

                match Arc::clone(&self).do_heartbeat().await {
                    Ok(()) => has_sent_heartbeat = true,
                    Err(e) => {
                        eprintln!("Error while heartbeating: {:?}", e);
                        should_kill = true;
                    }
                };

                // rust is a terrible language
                // if you put the kill channel in the above block, you are told e might be used later
                // even if you explicitly drop(e), it still says e is dropped after the `if let` scope ends.
                // found a compiler bug on my second day of rust, wahey!
                if should_kill {
                    // BIG problem
                    // TODO: panic?
                    if let Err(e) = self.kill_shard_tx.clone().send(()).await {
                        eprintln!("Failed to kill shard! {}", e);
                    }

                    break;
                }

                delay_for(interval).await;
            }
        });

        cancel_tx
    }

    async fn do_heartbeat(self: Arc<Self>) -> Result<(), GatewayError> {
        let payload: payloads::Heartbeat;
        {
            let seq = self.seq.read().await;
            payload = payloads::Heartbeat::new(*seq);
        }

        let (res_tx, res_rx) = oneshot::channel();

        self.write(payload, res_tx).await?;
        res_rx.await??;

        *self.last_heartbeat.write().await = Instant::now();

        Ok(())
    }

    async fn do_identify(self: Arc<Self>) -> Result<(), GatewayError> {
        self.wait_for_ratelimit().await?;

        let (tx, rx) = oneshot::channel();
        self.write(&self.identify, tx).await?;
        Ok(rx.await.map_err(GatewayError::RecvError)?.map_err(GatewayError::WebsocketError)?)
    }

    async fn wait_for_ratelimit(&self) -> Result<(), GatewayError> {
        let key = format!("ratelimiter:public:identify:{}", self.get_shard_id() % self.large_sharding_buckets);

        let mut res = darkredis::Value::Nil;
        while res == darkredis::Value::Nil {
            let mut conn = self.redis.get().await;

            // some arbitrary value, set if not exist, set expiry, of 6s
            res = conn.run_command(
                Command::new("SET")
                    .args(&[&key[..], "1", "NX", "PX", "6000"])
            ).await.map_err(GatewayError::RedisError)?;

            if res == darkredis::Value::Nil {
                // get time to delay
                let cmd = Command::new("PTTL").arg(&key);

                match conn.run_command(cmd).await.map_err(GatewayError::RedisError)? {
                    darkredis::Value::Integer(ttl) => {
                        // if number is negative, we can go ahead and identify
                        // -1 = no expire, -2 = doesn't exist
                        if ttl > 0 {
                            let ttl = Duration::from_millis(ttl as u64);
                            delay_for(ttl).await
                        }
                    },
                    _ => {}
                }
            }
        }

        Ok(())
    }

    /// Shard.session_id & Shard.seq should not be None when calling this function
    /// if they are, the function will panic
    async fn do_resume(self: Arc<Self>) -> Result<(), GatewayError> {
        let payload = payloads::Resume::new(
            self.identify.data.token.clone(),
            self.session_id.read().await.as_ref().unwrap().clone(),
            self.seq.read().await.unwrap(),
        );

        let (tx, rx) = oneshot::channel();
        self.write(payload, tx).await?;

        Ok(rx.await.map_err(GatewayError::RecvError)?.map_err(GatewayError::WebsocketError)?)
    }

    /// helper
    fn get_shard_id(&self) -> u16 {
        self.identify.data.shard_info.shard_id
    }
}