use std::time::Duration;

use kafka::{
    client::RequiredAcks, error::Error as KafkaError, producer::Producer, producer::Record,
};
use tokio::sync::mpsc;
use tracing::error;

use crate::{config::AppConfig, service::Request};

// producer is sends received Requests by topics.
pub async fn producer(
    mut reciver: mpsc::Receiver<(String, Vec<Request>)>,
    config: AppConfig,
) -> Result<(), KafkaError> {
    let mut producer = Producer::from_hosts(config.service.kafka_brokers)
        .with_ack_timeout(Duration::from_secs(1))
        .with_required_acks(RequiredAcks::One)
        .create()?;

    while let Some((topic, requests)) = reciver.recv().await {
        let s = match serde_json::to_string(&requests) {
            Ok(s) => s,
            Err(e) => {
                error!("request serialize: {}", e);
                continue;
            }
        };
        if let Err(e) = producer.send(&Record::from_value(topic.as_str(), s)) {
            error!("kafka error: {}", e)
        }
    }

    Ok(())
}
