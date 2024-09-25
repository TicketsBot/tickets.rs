use std::{sync::atomic::{AtomicUsize, Ordering}, time::Duration};

use common::event_forwarding;
use rdkafka::{error::KafkaError, producer::{BaseProducer, BaseRecord, Producer}, types::RDKafkaErrorCode, ClientConfig};
use crate::Result;

pub struct Publisher {
    topic: String,
    producer: BaseProducer,
    since_last_poll: AtomicUsize,
}

const POLL_INTERVAL: usize = 100;

impl Publisher {
    pub fn new(brokers: Vec<String>, topic: String) -> Result<Self> {
        let producer: BaseProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers.join(","))
            .create()?;

        Ok(Self { 
            topic, 
            producer,
            since_last_poll: AtomicUsize::new(0),
        })
    }

    pub fn send(&self, ev: &event_forwarding::Event, guild_id: u64) -> Result<()> {
        let marshalled = serde_json::to_vec(ev)?;

        let key = guild_id.to_string();
        let record = BaseRecord::to(&self.topic.as_str())
            .payload(&marshalled)
            .key(key.as_str());

        // Err is infallible
        _ = self.since_last_poll.compare_exchange(POLL_INTERVAL, 0, Ordering::Relaxed, Ordering::Relaxed);
        self.since_last_poll.fetch_add(1, Ordering::Relaxed);

        match self.producer.send(record) {
            Ok(_) => Ok(()),
            Err((e@KafkaError::MessageProduction(RDKafkaErrorCode::QueueFull), _)) => {
                self.producer.poll(Duration::from_millis(10));
                return Err(e.into());
            },
            Err((e, _)) => Err(e.into()),
        }
    }

    pub fn flush(&self, timeout: Duration) {
        self.producer.flush(timeout);
    }
}
