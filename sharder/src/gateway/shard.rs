use super::payloads;
use super::payloads::{Opcode, Payload};

use super::OutboundMessage;

use std::error::Error;
use std::str;

use flate2::{Decompress, FlushDecompress, Status};

use tokio::time::delay_for;
use std::time::Duration;

use std::sync::Arc;
use tokio::sync::RwLock;

use tokio::sync::{mpsc, oneshot};
use futures_util::{StreamExt, SinkExt, stream::{SplitSink, SplitStream}};
use tokio_tungstenite::{WebSocketStream, connect_async, tungstenite::Message};
use tokio::net::TcpStream;
use tokio_tls::TlsStream;
use tokio::sync::oneshot::error::RecvError;

pub struct Shard {
    seq: Arc<RwLock<Option<usize>>>,
    writer: Option<mpsc::Sender<OutboundMessage>>,
    kill_heartbeat: Option<oneshot::Sender<()>>,
    kill_shard: Option<mpsc::Sender<()>>,
}

// 16 KiB
const CHUNK_SIZE: usize = 16 * 1024;

type WebSocketTx = SplitSink<WebSocketStream<tokio_tungstenite::stream::Stream<TcpStream, TlsStream<TcpStream>>>, Message>;
type WebSocketRx = SplitStream<WebSocketStream<tokio_tungstenite::stream::Stream<TcpStream, TlsStream<TcpStream>>>>;

impl Shard {
    pub fn new() -> Shard {
        Shard {
            seq: Arc::new(RwLock::new(None)),
            writer: None,
            kill_heartbeat: None,
            kill_shard: None,
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

        let (kill_shard_tx, mut kill_shard_rx) = mpsc::channel(1);
        self.kill_shard = Some(kill_shard_tx);

        // start read loop
        self.listen(ws_rx, kill_shard_rx).await?;

        Ok(())
    }

    async fn handle_writes(mut ws_tx: WebSocketTx, mut rx: mpsc::Receiver<super::OutboundMessage>) {
        while let Some(msg) = rx.recv().await {
            println!("{}", msg.message);

            let payload = tokio_tungstenite::tungstenite::Message::text(msg.message);
            let res = ws_tx.send(payload).await;

            if let Err(e) = msg.tx.send(res) {
                eprintln!("Error while sending write result back to caller: {:?}", e);
            }
        }
    }

    async fn listen(&mut self, mut ws_rx: WebSocketRx, mut kill_shard_rx: mpsc::Receiver<()>) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        let mut decoder = Decompress::new(true);

        while let Err(mpsc::error::TryRecvError::Empty) = kill_shard_rx.try_recv() {
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

                let payload = Shard::read_payload(msg, &mut decoder).await;
                if payload.is_none() {
                    continue;
                }
                let (payload, raw) = payload.unwrap();

                self.process_payload(payload, raw).await;
            }
        }

        Ok(())
    }

    // we return None because it's ok to discard the payload
    async fn read_payload(msg: tokio_tungstenite::tungstenite::protocol::Message, decoder: &mut Decompress) -> Option<(Payload, Vec<u8>)> {
        let compressed = msg.into_data();

        let mut output: Vec<u8> = Vec::new();
        let mut offset: usize = 0;

        while (decoder.total_in() as usize) < compressed.len() {
            let mut temp: Vec<u8> = Vec::with_capacity(CHUNK_SIZE);

            match decoder.decompress_vec(&compressed[offset..], &mut temp, FlushDecompress::Sync) {
                Ok(Status::StreamEnd) => break,
                Ok(Status::Ok) | Ok(Status::BufError) => {
                    output.append(&mut temp);
                    offset = decoder.total_in() as usize;
                }

                Err(e) => {
                    eprintln!("Error while decompressing: {}", e);
                    return None;
                }
            };
        }

        // TODO: REMOVE DEBUG
        println!("{:?}", str::from_utf8(&output).unwrap());

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
            Opcode::Hello => {
                let hello: payloads::Hello = serde_json::from_slice(&raw[..])?;

                self.do_identify().await;

                // we unwrap here because writer should never be None.
                let kill_tx = Shard::start_heartbeat(
                    hello.data.heartbeat_interval,
                    self.writer.as_ref().unwrap().clone(),
                    self.kill_shard.as_ref().unwrap().clone(),
                    Arc::clone(&self.seq)
                ).await;

                self.kill_heartbeat = Some(kill_tx);

                Ok(())
            }
            _ => Ok(()),
        }
    }

    // returns cancellation channel
    async fn start_heartbeat(
        interval: u32,
        writer_tx: mpsc::Sender<OutboundMessage>,
        mut kill_shard_tx: mpsc::Sender<()>,
        seq: Arc<RwLock<Option<usize>>>,
    ) -> oneshot::Sender<()> {
        let interval = Duration::from_millis(interval as u64);

        let (cancel_tx, mut cancel_rx) = oneshot::channel();

        tokio::spawn(async move {
            delay_for(interval).await;

            while let Err(oneshot::error::TryRecvError::Empty) = cancel_rx.try_recv() {
                let mut should_kill = false;

                if let Err(e) = Shard::do_heartbeat(writer_tx.clone(), Arc::clone(&seq)).await {
                    eprintln!("Error while heartbeating: {:?}", e);
                    should_kill = true;
                    break
                }

                // rust is a terrible language
                // if you put the kill channel in the above block, you are told e might be used later
                // even if you explicitly drop(e), it still says e is dropped after the `if let` scope ends.
                // found a compiler bug on my second day of rust, wahey!
                if should_kill {
                    kill_shard_tx.send(()).await;
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
    async fn do_identify(&mut self) {
        
    }
}