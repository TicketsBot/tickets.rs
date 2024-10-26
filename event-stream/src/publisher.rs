use std::{sync::atomic::{AtomicUsize, Ordering}, time::Duration};

use common::event_forwarding;
use rdkafka::{error::KafkaError, producer::{BaseProducer, BaseRecord, Producer}, types::RDKafkaErrorCode, ClientConfig};
use crate::Result;

#[cfg(feature = "metrics")]
use {
    std::time::{SystemTime, UNIX_EPOCH},
    lazy_static::lazy_static,
};

pub struct Publisher {
    topic: String,
    producer: BaseProducer,
    since_last_poll: AtomicUsize,
    #[cfg(feature = "metrics")]
    last_flush: AtomicUsize,
}

const POLL_INTERVAL: usize = 100;

#[cfg(feature = "metrics")]
lazy_static! {
    static ref FLUSH_INTERVAL_HISTOGRAM: prometheus::Histogram = prometheus::register_histogram!(
        "kafka_flush_interval",
        "Time between Kafka flushes",
    )
    .unwrap();
}

impl Publisher {
    pub fn new(brokers: Vec<String>, topic: String) -> Result<Self> {
        let producer: BaseProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers.join(","))
            .create()?;

        Ok(Self { 
            topic, 
            producer,
            since_last_poll: AtomicUsize::new(0),
            #[cfg(feature = "metrics")]
            last_flush: AtomicUsize::new(0),
        })
    }

    pub fn send(&self, ev: &event_forwarding::Event, _guild_id: u64) -> Result<()> {
        let marshalled = serde_json::to_vec(ev)?;

        let record: BaseRecord<'_, String, Vec<u8>> = BaseRecord::to(&self.topic.as_str())
            .payload(&marshalled);

        let should_poll = self.since_last_poll.compare_exchange(POLL_INTERVAL, 0, Ordering::Relaxed, Ordering::Relaxed).is_ok();
        self.since_last_poll.fetch_add(1, Ordering::Relaxed);

        if should_poll {
            self.producer.poll(Duration::from_millis(100));
            
            #[cfg(feature = "metrics")]
            {
                let now = get_unix_millis();
                let last_flush = self.last_flush.swap(now, Ordering::Relaxed);

                if last_flush != 0 {
                    let elapsed = now - last_flush;
                    FLUSH_INTERVAL_HISTOGRAM.observe(elapsed as f64);
                }
            }
        }

        match self.producer.send(record) {
            Ok(_) => Ok(()),
            Err((e@KafkaError::MessageProduction(RDKafkaErrorCode::QueueFull), _)) => {
                self.producer.poll(Duration::from_millis(100));
                return Err(e.into());
            },
            Err((e, _)) => Err(e.into()),
        }
    }

    pub fn flush(&self, timeout: Duration) {
        self.producer.flush(timeout);
    }
}

#[cfg(feature = "metrics")]
fn get_unix_millis() -> usize {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis().try_into().unwrap()
}
