use futures::future::try_join_all;
use tokio::sync::mpsc;
use tracing::error;

use crate::config::ServiceConfig;

use super::{store::Store, Request};

// process is handler for storing receiving Requests into Store and sending
// them to Kafka sender if chunk is full or by timer.
pub async fn process(
    mut receiver: mpsc::Receiver<Request>,
    kafka_sender: mpsc::Sender<(String, Vec<Request>)>,
    config: ServiceConfig,
) {
    let mut store = Store::new(&config);

    let mut delay = tokio::time::interval(config.max_collect_chunk_duration.into());

    loop {
        tokio::select! {
            Some(msg) = receiver.recv() => {
                if let Some((topic, requests)) = store.push(msg.clone()) {
                    if let Err(e) = kafka_sender.send((topic.clone(), requests)).await
                    {
                        error!("sender: {}", e)
                    }
                    delay.reset();
                };
            }

            _ = delay.tick() => {
                let fs = store
                    .pop_all()
                    .iter_mut()
                    .filter(|(_, requests)| !requests.is_empty())
                    .map(|(topic, requests)| kafka_sender.send((topic.clone(), requests.clone())))
                    .collect::<Vec<_>>();

                if let Err(e) = try_join_all(fs).await {
                    error!("sender: {}", e)
                }
            }
        }
    }
}
