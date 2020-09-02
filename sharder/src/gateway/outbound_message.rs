use serde::Serialize;
use tokio::sync::{oneshot, mpsc};
use tokio_tungstenite::tungstenite::Error;

#[derive(Debug)]
pub struct OutboundMessage {
    pub message: String,
    pub tx: oneshot::Sender<Result<(), Error>>,
}

impl OutboundMessage {
    pub fn new<T: Serialize>(msg: T, tx: oneshot::Sender<Result<(), Error>>) -> Result<OutboundMessage, serde_json::Error> {
        let serialized = serde_json::to_string(&msg)?;

        Ok(OutboundMessage {
            message: serialized,
            tx,
        })
    }

    pub async fn send(self, mut tx: mpsc::Sender<OutboundMessage>) -> Result<(), mpsc::error::SendError<OutboundMessage>> {
        tx.send(self).await
    }
}