use std::sync::Arc;

use ::tracing::info;
use tokio::signal;
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

struct TickTock {
    tick_sender: mpsc::Sender<()>,
    tick_receiver: mpsc::Receiver<()>,
    tock_sender: mpsc::Sender<()>,
    tock_receiver: mpsc::Receiver<()>,
}

impl TickTock {
    fn new() -> TickTock {
        let (tick_sender, tick_receiver) = mpsc::channel::<()>(1);
        let (tock_sender, tock_receiver) = mpsc::channel::<()>(1);
        TickTock {
            tick_sender,
            tick_receiver,
            tock_sender,
            tock_receiver,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    info!("start application");

    let app_config = config::AppConfig::load()?;
    info!(
        "config loaded:\n{:?}",
        serde_json::to_string_pretty(&app_config)?
    );

    let metrics = init_api_metrics();

    let mut tick_tock = TickTock::new();

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
    tokio::spawn(async move {
        service::process(
            store,
            kafka_sender,
            cfg,
            tick_tock.tick_receiver,
            tick_tock.tock_sender,
        )
        .await
    });

    let server = init_server(app_config.server, metrics, data)?;
    let srv_handle = server.handle();

    tokio::spawn(async move { server.await });

    match signal::ctrl_c().await {
        Ok(()) => {
            srv_handle.stop(true).await;
            info!("server stop");
            tick_tock.tick_sender.send(()).await?;
            tick_tock.tock_receiver.recv().await;
            info!("process stop");

            let mut delay = tokio::time::interval(tokio::time::Duration::from_secs(5));
            loop {
                tokio::select! {
                    _ = tick_tock.tock_receiver.recv() => {
                        info!("graceful shutdown successfully");
                        break
                    }

                    _ = delay.tick() => {
                        return Err(anyhow::anyhow!("graceful shutdown failed"))
                    }
                }
            }

            anyhow::Ok(())
        }
        Err(e) => Err(anyhow::anyhow!(e)),
    }
}
