use super::payloads;
use super::payloads::{Opcode, Payload};
use super::payloads::event::Event;

use super::OutboundMessage;

use std::error::Error;
use std::str;

use flate2::{Decompress, FlushDecompress, Status};

use tokio::time::delay_for;
use std::time::Duration;
use std::time::Instant;

use std::sync::Arc;
use tokio::sync::RwLock;

use tokio::sync::{mpsc, oneshot};
use futures_util::{StreamExt, SinkExt, stream::{SplitSink, SplitStream}};
use tokio_tungstenite::{WebSocketStream, connect_async, tungstenite::Message};
use tokio::net::TcpStream;
use tokio_tls::TlsStream;
use serde::Serialize;

pub struct Shard {
    identify: payloads::Identify,
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
    pub fn new(identify: payloads::Identify) -> Shard {
        let (kill_shard_tx, kill_shard_rx) = mpsc::channel(1);

        Shard {
            identify,
            total_rx: 0,
            seq: Arc::new(RwLock::new(None)),
            session_id: Arc::new(RwLock::new((None))),
            writer: None,
            kill_heartbeat: None,
            kill_shard_tx,
            kill_shard_rx,
            last_ack: Arc::new(RwLock::new(Instant::now())),
        }
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        let uri = url::Url::parse("wss://gateway.discord.gg/?v=6&encoding=json&compress=zlib-stream").unwrap();

        let (wss, _) = connect_async(uri).await?;
        let (ws_tx, ws_rx) = wss.split();

        // start writer
        let (writer_tx, writer_rx) = mpsc::channel(100);
        tokio::spawn(async move {
            Shard::handle_writes(ws_tx, writer_rx).await;
        });
        self.writer = Some(writer_tx);

        // start read loop
        self.listen(ws_rx).await?;

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
        tx: oneshot::Sender<Result<(), tokio_tungstenite::tungstenite::Error>>
    ) -> Result<(), Box<dyn Error>> {
        OutboundMessage::new(msg, tx)?.send(self.writer.clone().unwrap()).await?;
        Ok(())
    }

    // helper function
    async fn kill(&mut self) {
        self.kill_shard_tx.send(()).await;
    }

    async fn listen(&mut self, mut ws_rx: WebSocketRx) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        let mut decoder = Decompress::new(true);

        while let Err(mpsc::error::TryRecvError::Empty) = self.kill_shard_rx.try_recv() {
            while let Some(msg) = ws_rx.next().await {
                if let Err(e) = msg {
                    return Err(e);
                }

                let msg = msg.unwrap();

                if msg.is_close() {
                    return Ok(());
                }

                if !msg.is_binary() {
                    eprintln!("Received non-binary message");
                    continue;
                }

                let payload = self.read_payload(msg, &mut decoder).await;
                if payload.is_none() {
                    continue;
                }
                let (payload, raw) = payload.unwrap();

                if let Err(e) = self.process_payload(payload, raw).await {
                    eprintln!("Error reading payload: {}", e);
                }
            }
        }

        Ok(())
    }

    // we return None because it's ok to discard the payload
    async fn read_payload(&mut self, msg: tokio_tungstenite::tungstenite::protocol::Message, decoder: &mut Decompress) -> Option<(Payload, Vec<u8>)> {
        let compressed = msg.into_data();

        let mut output: Vec<u8> = Vec::new();
        let mut offset: usize = 0;

        while ((decoder.total_in() - self.total_rx) as usize) < compressed.len() {
            let mut temp: Vec<u8> = Vec::with_capacity(CHUNK_SIZE);

            match decoder.decompress_vec(&compressed[offset..], &mut temp, FlushDecompress::Sync) {
                Ok(Status::StreamEnd) => break,
                Ok(Status::Ok) | Ok(Status::BufError) => {
                    output.append(&mut temp);
                    offset = decoder.total_in() as usize;
                }

                // TODO: Should we reconnect?
                Err(e) => {
                    eprintln!("Error while decompressing: {}", e);
                    self.total_rx = decoder.total_in();
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
                let payload: payloads::Dispatch = serde_json::from_slice(&raw[..])?;
                self.handle_event(payload.data).await;
                Ok(())
            },
            Opcode::Reconnect => {
                self.kill();
                Ok(())
            },
            Opcode::InvalidSession => {
                let res: Result<payloads::InvalidSession, serde_json::Error> = serde_json::from_slice(&raw[..]);

                match &res {
                    Ok(payload) => {
                        if !payload.is_resumable {
                            *self.session_id.write().await = None;
                        }
                    },
                    Err(e) => {
                        *self.session_id.write().await = None;
                    },
                };

                res.map(|_| ()).map_err(|e| Box::new(e) as Box<dyn Error>)
            }
            Opcode::Hello => {
                let hello: payloads::Hello = serde_json::from_slice(&raw[..])?;

                if self.session_id.read().await.is_some() && self.seq.read().await.is_some() {
                    self.do_resume().await;
                } else {
                    self.do_identify().await;
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
            },
            Opcode::HeartbeatAck => {
                let mut last_ack = self.last_ack.write().await;
                *last_ack = Instant::now();

                Ok(())
            }
            _ => Ok(()),
        }
    }

    async fn handle_event(&mut self, event: Event) {
        match event {
            Event::Ready(ready) => {
                *self.session_id.write().await = Some(ready.session_id);
                println!("Connected on {}#{} ({}) shard {} / {}", ready.user.username, ready.user.discriminator, ready.user.id, ready.shard.shard_id, ready.shard.num_shards);
            },
            Event::ChannelCreate(channel) => {
                println!("{:?}", channel);
            }
            _ => {},
        };
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
                        kill_shard_tx.send(()).await;
                        break
                    }
                }

                let mut should_kill = false;

                match Shard::do_heartbeat(writer_tx.clone(), Arc::clone(&seq)).await {
                    Ok(()) => has_sent_heartbeat = true,
                    Err(e) => {
                        eprintln!("Error while heartbeating: {:?}", e);
                        should_kill = true;
                    },
                };

                // rust is a terrible language
                // if you put the kill channel in the above block, you are told e might be used later
                // even if you explicitly drop(e), it still says e is dropped after the `if let` scope ends.
                // found a compiler bug on my second day of rust, wahey!
                if should_kill {
                    kill_shard_tx.send(()).await;
                    break
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

    // TODO: Ratelimit
    async fn do_identify(&mut self) -> Result<(), Box<dyn Error>> {
        let (tx, rx) = oneshot::channel();
        self.write(&self.identify, tx).await?;
        Ok(rx.await??)
    }

    /// Shard.session_id & Shard.seq should not be None when calling this function
    /// if they are, the function will panic
    async fn do_resume(&mut self) -> Result<(), Box<dyn Error>> {
        let payload = payloads::Resume::new(
            self.identify.data.token.clone(),
            self.session_id.read().await.as_ref().unwrap().clone(),
            self.seq.read().await.unwrap()
        );

        let (tx, rx) = oneshot::channel();
        self.write(payload, tx).await;

        Ok(rx.await??)
    }
}