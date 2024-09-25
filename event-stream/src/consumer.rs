use common::event_forwarding::Event;
use rdkafka::{consumer::{Consumer as KafkaConsumer, StreamConsumer}, ClientConfig, Message};
use tracing::warn;
use crate::Result;

pub struct Consumer {
    consumer: StreamConsumer,
}

impl Consumer {
    pub fn new(brokers: Vec<String>, topic: String, group_id: String) -> Result<Self> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", brokers.join(","))
            .set("group.id", group_id)
            .create()?;

        consumer.subscribe(&[topic.as_str()])?;

        Ok(Self {
            consumer,
        })
    }

    pub async fn recv(&self) -> Result<Event> {
        let msg = self.consumer.recv().await?;

        let json = match msg.payload_view::<[u8]>() {
            Some(v) => v,
            None => {
                warn!("Received message with no payload. Ignoring.");
                return Box::pin(self.recv()).await;
            }
        }.expect("Infallible");
        
        serde_json::from_slice(json).map_err(Into::into)
    }
}