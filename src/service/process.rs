use std::sync::Arc;

use futures::future::try_join_all;
use tokio::sync::mpsc;
use tracing::{debug, error};

use crate::config::ServiceConfig;

use super::{store::Store, Request};

// process is handler for storing receiving Requests into Store and sending
// them to Kafka sender if chunk is full or by timer.
pub async fn process(
    store: Arc<Store>,
    kafka_sender: mpsc::Sender<(String, Vec<Request>)>,
    config: ServiceConfig,
) {
    let mut delay = tokio::time::interval(config.max_collect_chunk_duration.into());

    loop {
        tokio::select! {
            _ = delay.tick() => {
                match store.pop_all() {
                    Ok(mut c) => {
                        let fs = c
                            .iter_mut()
                            .filter(|(_, requests)| !requests.is_empty())
                            .map(|(topic, requests)| {
                                requests.iter().for_each(|r| debug!("{}", serde_json::to_string(&r).expect("Error occurred while serializing Request to JSON")));
                                kafka_sender.send((topic.clone(), requests.clone()))
                            })
                            .collect::<Vec<_>>();

                        if let Err(e) = try_join_all(fs).await {
                            error!("kafka_sender: {}", e)
                        }
                    }
                    Err(e) => error!("{}", e),
                }
            }
        }
    }
}
