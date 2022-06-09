use std::time::Duration;

use kafka::{
    client::RequiredAcks, error::Error as KafkaError, producer::Producer, producer::Record,
};
use tokio::sync::mpsc;
use tracing::error;

use crate::{config::AppConfig, service::Request};

pub struct Service {
    producer: Producer,
    receiver: mpsc::Receiver<(String, Vec<Request>)>,
}

impl Service {
    pub fn new(
        config: &AppConfig,
        receiver: mpsc::Receiver<(String, Vec<Request>)>,
    ) -> Result<Self, KafkaError> {
        let srv = Service {
            producer: Producer::from_hosts(config.kafka.brokers.clone())
                .with_ack_timeout(match config.kafka.ack_timeout {
                    Some(d) => d.into(),
                    None => Duration::from_secs(1),
                })
                .with_required_acks(RequiredAcks::One)
                .create()?,
            receiver,
        };

        Ok(srv)
    }

    // run is sends received Requests by topics.
    pub async fn run(&mut self) {
        while let Some((topic, requests)) = self.receiver.recv().await {
            let s = match serde_json::to_string(&requests) {
                Ok(s) => s,
                Err(e) => {
                    error!("request serialize: {}", e);
                    continue;
                }
            };
            if let Err(e) = self.producer.send(&Record::from_value(topic.as_str(), s)) {
                error!("kafka error: {}", e)
            }
        }
    }
}
