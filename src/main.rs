use ::tracing::info;
use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::{mpsc, oneshot};

use crate::metrics::init_api_metrics;
use crate::tracing::init_tracing;

mod config;
mod kafka;
mod metrics;
mod server;
mod service;
mod tracing;

struct TickTock {
    tick_sender: oneshot::Sender<()>,
    tick_receiver: oneshot::Receiver<()>,
    tock_sender: oneshot::Sender<()>,
    tock_receiver: oneshot::Receiver<()>,
}

impl TickTock {
    fn new() -> TickTock {
        let (tick_sender, tick_receiver) = oneshot::channel();
        let (tock_sender, tock_receiver) = oneshot::channel();
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

    let tick_tock = TickTock::new();

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

    let server = server::Server::run(app_config.server, metrics, data)?;

    graceful_chutdown(server, tick_tock.tick_sender, tick_tock.tock_receiver).await
}

async fn graceful_chutdown(
    server: server::Server,
    tick_sender: oneshot::Sender<()>,
    tock_receiver: oneshot::Receiver<()>,
) -> anyhow::Result<()> {
    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigterm = signal(SignalKind::terminate())?;
    tokio::select! {
        _ = sigint.recv() => {},
        _ = sigterm.recv() =>{}
    }

    info!("graceful shutdown started");
    let (shutdown_sender, shutdown_receiver) = oneshot::channel();
    tokio::spawn(async move {
        let sleep = tokio::time::sleep(tokio::time::Duration::from_secs(4));
        tokio::pin!(sleep);

        tokio::select! {
            _ = shutdown_receiver => {}

            _ = &mut sleep => {
                panic!("graceful shutdown failed");
            }
        }
    });

    server.stop().await;
    info!("server stopped");
    if tick_sender.send(()).is_err() {
        return Err(anyhow::anyhow!("tick_sender.send() is failed"));
    };
    if let Err(e) = tock_receiver.await {
        return Err(anyhow::anyhow!("tick_tock.tock_receiver: {}", e));
    };
    info!("process stoped");
    if shutdown_sender.send(()).is_err() {
        return Err(anyhow::anyhow!("shutdown_receiver.send() failed"));
    }
    info!("graceful shutdown successfully");

    anyhow::Ok(())
}
