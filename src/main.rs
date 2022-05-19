use ::tracing::{error, info};
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
    let cfg = app_config.clone();
    tokio::spawn(async move {
        if let Err(e) = kafka::producer(kafka_receiver, cfg).await {
            error!("kafka producer: {}", e)
        }
    });

    let (request_sender, request_receiver) = mpsc::channel::<service::Request>(32);
    let cfg = app_config.service.clone();
    tokio::spawn(async move { service::process(request_receiver, kafka_sender, cfg).await });

    let data = server::AppState {
        config: app_config.clone(),
        sender: request_sender,
    };

    let s = init_server(app_config.server, init_api_metrics(), data)?;
    match s.await {
        Err(e) => Err(anyhow::anyhow!("{}", e)),
        _ => anyhow::Ok(()),
    }
}
