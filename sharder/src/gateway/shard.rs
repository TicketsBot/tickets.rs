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
use tokio::sync::RwLock;

use r2d2::Pool;

use tokio::sync::{mpsc, oneshot};
use futures_util::{StreamExt, SinkExt, stream::{SplitSink, SplitStream}};
use tokio_tungstenite::{WebSocketStream, connect_async, tungstenite::Message};
use tokio::net::TcpStream;
use tokio_tls::TlsStream;
use serde::Serialize;
use cache::{Cache, PostgresCache};
use model::guild::Member;
use chrono::Utc;
use std::ops::{Sub, DerefMut};
use model::channel::Channel;
use model::Snowflake;
use common::event_forwarding;
use crate::gateway::payloads::Dispatch;
use serde_json::{Value, Map};
use r2d2_redis::{RedisConnectionManager, redis};
use crate::gateway::GatewayError;
use crate::manager::FatalError;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

pub struct Shard {
    identify: payloads::Identify,
    large_sharding_buckets: u16,
    cache: Arc<PostgresCache>,
    redis: Arc<Pool<RedisConnectionManager>>,
    is_whitelabel: bool,
    error_tx: mpsc::Sender<FatalError>,
    bot_id: RwLock<Option<Snowflake>>,
    total_rx: u64,
    seq: Arc<RwLock<Option<usize>>>,
    session_id: Arc<RwLock<Option<String>>>,
    writer: Option<mpsc::Sender<OutboundMessage>>,
    kill_heartbeat: Option<oneshot::Sender<()>>,
    kill_shard_tx: mpsc::Sender<()>,
    kill_shard_rx: mpsc::Receiver<()>,
    last_ack: Arc<RwLock<Instant>>,
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
        redis: Arc<Pool<RedisConnectionManager>>,
        is_whitelabel: bool,
        error_tx: mpsc::Sender<FatalError>
    ) -> Shard {
        let (kill_shard_tx, kill_shard_rx) = mpsc::channel(1);

        Shard {
            identify,
            large_sharding_buckets,
            cache,
            redis,
            is_whitelabel,
            error_tx,
            bot_id: RwLock::new(None),
            total_rx: 0,
            seq: Arc::new(RwLock::new(None)),
            session_id: Arc::new(RwLock::new(None)),
            writer: None,
            kill_heartbeat: None,
            kill_shard_tx,
            kill_shard_rx,
            last_ack: Arc::new(RwLock::new(Instant::now())),
        }
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        self.total_rx = 0; // rst

        let uri = url::Url::parse("wss://gateway.discord.gg/?v=6&encoding=json&compress=zlib-stream").unwrap();

        let (wss, _) = connect_async(uri).await?;
        let (ws_tx, ws_rx) = wss.split();

        // start writer
        let (writer_tx, writer_rx) = mpsc::channel(16);
        tokio::spawn(async move {
            Shard::handle_writes(ws_tx, writer_rx).await;
        });
        self.writer = Some(writer_tx);

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
            .send(self.writer.clone().unwrap())
            .await
            .map_err(GatewayError::SendMessageError)?;

        Ok(())
    }

    // helper function
    async fn kill(&mut self) {
        // BIG problem
        // TODO: panic?
        if let Err(e) = self.kill_shard_tx.send(()).await {
            eprintln!("Failed to kill shard! {}", e);
        }
    }

    async fn listen(&mut self, mut ws_rx: WebSocketRx) -> Result<(), GatewayError> {
        let mut decoder = Decompress::new(true);

        loop {
            match self.kill_shard_rx.try_recv() {
                Err(mpsc::error::TryRecvError::Empty) => {}
                _ => {
                    break;
                }
            };

            match ws_rx.next().await {
                None => {
                    println!("Shard {} closed (next is None)", self.get_shard_id());
                    break;
                }

                Some(Err(e)) => {
                    return Err(GatewayError::WebsocketError(e));
                }

                Some(Ok(msg)) => {
                    if let Message::Close(frame) = msg {
                        let frame = frame.unwrap_or(CloseFrame {
                            code: CloseCode::Invalid,
                            reason: Default::default(),
                        });

                        let wrapped = FatalError::new(self.identify.data.token.clone(), frame.code, frame.reason.to_string());

                        if let Err(e) = self.error_tx.send(wrapped).await {
                            return Err(GatewayError::SendErrorError(e))
                        }

                        return Ok(());
                    }

                    if !msg.is_binary() {
                        eprintln!("Received non-binary message");
                        continue;
                    }

                    let (payload, raw) = match self.read_payload(msg, &mut decoder).await {
                        Some(r) => r,
                        _ => continue,
                    };

                    if let Err(e) = self.process_payload(payload, raw).await {
                        eprintln!("Error reading payload: {}", e);
                    }
                }
            };
        }

        Ok(())
    }

    // we return None because it's ok to discard the payload
    async fn read_payload(&mut self, msg: tokio_tungstenite::tungstenite::protocol::Message, decoder: &mut Decompress) -> Option<(Payload, Vec<u8>)> {
        let compressed = msg.into_data();

        let mut output: Vec<u8> = Vec::with_capacity(CHUNK_SIZE);
        let before = self.total_rx;
        let mut offset: usize = 0;

        while ((decoder.total_in() - self.total_rx) as usize) < compressed.len() {
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

                    self.total_rx = 0;
                    decoder.reset(true);

                    return None;
                }
            };
        }

        self.total_rx = decoder.total_in();

        // deserialize payload
        match serde_json::from_slice(&output[..]) {
            Ok(payload) => Some((payload, output)),
            Err(e) => {
                eprintln!("Error occurred while deserializing payload: {}", e);
                None
            }
        }
    }

    async fn process_payload(&mut self, payload: Payload, raw: Vec<u8>) -> Result<(), Box<dyn Error>> {
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
                let payload: Map<String, Value> = serde_json::from_slice(&raw[..])?;
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

                res.map(|_| ()).map_err(|e| Box::new(e) as Box<dyn Error>)
            }
            Opcode::Hello => {
                let hello: payloads::Hello = serde_json::from_slice(&raw[..])?;

                let res = if self.session_id.read().await.is_some() && self.seq.read().await.is_some() {
                    self.do_resume().await
                } else {
                    self.do_identify().await
                };

                // TODO: Fatal error log, check if we should remove token
                if let Err(e) = res {
                    eprintln!("Received error while authenticating: {}", e);
                    self.kill().await;
                    return Err(Box::new(e));
                }

                // we unwrap here because writer should never be None.
                let kill_tx = Shard::start_heartbeat(
                    hello.data.heartbeat_interval,
                    self.writer.as_ref().unwrap().clone(),
                    self.kill_shard_tx.clone(),
                    Arc::clone(&self.seq),
                    Arc::clone(&self.last_ack),
                ).await;

                self.kill_heartbeat = Some(kill_tx);

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

    async fn handle_event(&mut self, data: Map<String, Value>) -> Result<(), GatewayError> {
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
            bot_id,
            is_whitelabel: self.is_whitelabel,
            shard_id: self.get_shard_id(),
            event_type: event_type.to_owned(),
            data: payload.data.get("d").unwrap(),
            extra,
        };

        let json = serde_json::to_string(&wrapped).map_err(GatewayError::JsonError)?;
        redis::cmd("RPUSH")
            .arg(common::event_forwarding::KEY)
            .arg(json)
            .execute(self.redis.get().map_err(GatewayError::PoolError)?.deref_mut());

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
        interval: u32,
        writer_tx: mpsc::Sender<OutboundMessage>,
        mut kill_shard_tx: mpsc::Sender<()>,
        seq: Arc<RwLock<Option<usize>>>,
        last_ack: Arc<RwLock<Instant>>,
    ) -> oneshot::Sender<()> {
        let interval = Duration::from_millis(interval as u64);

        let (cancel_tx, mut cancel_rx) = oneshot::channel();

        tokio::spawn(async move {
            delay_for(interval).await;

            let mut has_sent_heartbeat = false;
            while let Err(oneshot::error::TryRecvError::Empty) = cancel_rx.try_recv() {
                // check if we've received an ack
                {
                    let last_ack = last_ack.read().await;
                    if has_sent_heartbeat && last_ack.elapsed() > interval {
                        // BIG problem
                        // TODO: panic?
                        if let Err(e) = kill_shard_tx.send(()).await {
                            eprintln!("Failed to kill shard! {}", e);
                        }

                        break;
                    }
                }

                let mut should_kill = false;

                match Shard::do_heartbeat(writer_tx.clone(), Arc::clone(&seq)).await {
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
                    if let Err(e) = kill_shard_tx.send(()).await {
                        eprintln!("Failed to kill shard! {}", e);
                    }

                    break;
                }

                delay_for(interval).await;
            }
        });

        cancel_tx
    }

    async fn do_heartbeat(
        writer_tx: mpsc::Sender<OutboundMessage>,
        seq: Arc<RwLock<Option<usize>>>,
    ) -> Result<(), Box<dyn Error>> {
        let payload: payloads::Heartbeat;
        {
            let seq = seq.read().await;
            payload = payloads::Heartbeat::new(*seq);
        }

        let (res_tx, res_rx) = oneshot::channel();

        OutboundMessage::new(payload, res_tx)?.send(writer_tx).await?;
        res_rx.await??;

        Ok(())
    }

    async fn do_identify(&mut self) -> Result<(), GatewayError> {
        self.wait_for_ratelimit().await?;

        let (tx, rx) = oneshot::channel();
        self.write(&self.identify, tx).await?;
        Ok(rx.await.map_err(GatewayError::RecvError)?.map_err(GatewayError::WebsocketError)?)
    }

    async fn wait_for_ratelimit(&self) -> Result<(), GatewayError> {
        let key = &format!("ratelimiter:public:identify:{}", self.get_shard_id() % self.large_sharding_buckets);

        let mut res: Option<()> = None;
        while res.is_none() {
            res = redis::cmd("SET")
                .arg(key)
                .arg("1") // some arbitrary value
                .arg("NX") // set if key doesn't exist
                .arg("PX") // set expiry (millis)
                .arg("6000") // expiry in 6s
                .query(self.redis.get().map_err(GatewayError::PoolError)?.deref_mut()) // TODO: Use asyncio
                .map_err(GatewayError::RedisError)?;

            delay_for(Duration::from_secs(1)).await;
        }

        Ok(())
    }

    /// Shard.session_id & Shard.seq should not be None when calling this function
    /// if they are, the function will panic
    async fn do_resume(&mut self) -> Result<(), GatewayError> {
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