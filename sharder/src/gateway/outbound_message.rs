use crate::Result;
use serde::Serialize;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
pub struct OutboundMessage {
    pub message: String,
    pub tx: oneshot::Sender<Result<()>>,
}

impl OutboundMessage {
    pub fn new<T: Serialize>(
        msg: T,
        tx: oneshot::Sender<Result<()>>,
    ) -> Result<OutboundMessage, serde_json::Error> {
        let serialized = serde_json::to_string(&msg)?;

        Ok(OutboundMessage {
            message: serialized,
            tx,
        })
    }

    pub async fn send(
        self,
        tx: mpsc::Sender<OutboundMessage>,
    ) -> Result<(), mpsc::error::SendError<OutboundMessage>> {
        tx.send(self).await
    }
}
