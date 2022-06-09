use std::sync::Arc;

use futures::future::try_join_all;
use tokio::sync::mpsc;
use tracing::{debug, error};

use crate::{config::ServiceConfig, ticktock::Shutdowner};
use super::{store::Store, Request};

// Process is processing sending Requests to Kafka by timer.
pub async fn process(
    store: Arc<Store>,
    kafka_sender: mpsc::Sender<(String, Vec<Request>)>,
    config: ServiceConfig,
    shutdowner: Arc<Shutdowner>,
) {
    let mut delay = tokio::time::interval(config.max_collect_chunk_duration.into());

    let mut stopper = shutdowner
        .stop_handle()
        .await
        .expect("Error occurred while extracting stopper from shutdowner");

    tokio::select! {
        _ = async {
            loop {
                delay.tick().await;
                pop_all(store.clone(), kafka_sender.clone()).await;
            }
        } => {}
        _ = &mut stopper => {
            debug!("handle stop signal");
            pop_all(store.clone(), kafka_sender.clone()).await;
            shutdowner.stopped().await;
        }
    }
}

async fn pop_all(store: Arc<Store>, kafka_sender: mpsc::Sender<(String, Vec<Request>)>) {
    match store.pop_all() {
        Ok(mut c) => {
            let fs = c
                .iter_mut()
                .filter(|(_, requests)| !requests.is_empty())
                .map(|(topic, requests)| {
                    requests.iter().for_each(|r| {
                        debug!(
                            "{}",
                            serde_json::to_string(&r)
                                .expect("Error occurred while serializing Request to JSON")
                        )
                    });
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
