use std::sync::Arc;

use ::tracing::info;
use anyhow::Ok;
use tokio::sync::mpsc;

use crate::metrics::init_api_metrics;
use crate::server::init_server;
use crate::tracing::init_tracing;

mod config;
mod kafka;
mod metrics;
mod server;
mod service;
mod tracing;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    info!("start application");

    let app_config = config::AppConfig::load()?;
    info!(
        "config loaded:\n{:?}",
        serde_json::to_string_pretty(&app_config)?
    );

    let (kafka_sender, kafka_receiver) = mpsc::channel::<(String, Vec<service::Request>)>(32);
    let mut kafka_producer = kafka::Service::new(&app_config, kafka_receiver)?;
    tokio::spawn(async move { kafka_producer.run().await });

    let cfg = app_config.service.clone();
    let store = Arc::new(service::Store::new(&app_config.service));

    let data = server::AppState {
        config: app_config.clone(),
        sender: kafka_sender.clone(),
        store: store.clone(),
    };
    tokio::spawn(async move { service::process(store, kafka_sender, cfg).await });

    init_server(app_config.server, init_api_metrics(), data)?.await?;

    Ok(())
}
