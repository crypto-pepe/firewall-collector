use ::tracing::info;
use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;

use crate::ticktock::Shutdowner;
use crate::tracing::init_tracing;

mod config;
mod kafka;
mod metrics;
mod server;
mod service;
mod ticktock;
mod tracing;

static SHUTDOWN_INTERVAL_SEC: u64 = 5;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    info!("start application");

    let app_config =
        config::AppConfig::load().map_err(|e| anyhow::anyhow!("config load: {}", e))?;
    info!(
        "config loaded:\n{:?}",
        serde_json::to_string_pretty(&app_config)?
    );

    let shutdowner = Arc::new(Shutdowner::new());

    let (kafka_sender, kafka_receiver) = mpsc::channel::<(String, Vec<service::Request>)>(32);
    let mut kafka_producer = kafka::Service::new(&app_config, kafka_receiver)
        .map_err(|e| anyhow::anyhow!("kafka producer: {}", e))?;
    let kafka_handle = tokio::spawn(async move { kafka_producer.run().await });

    let cfg = app_config.service.clone();
    let store = Arc::new(service::Store::new(&app_config.service));

    let data = server::AppState {
        config: app_config.clone(),
        sender: kafka_sender.clone(),
        store: store.clone(),
    };
    let process_handle = {
        let tick_tock = shutdowner.clone();
        tokio::spawn(async move { service::process(store, kafka_sender, cfg, tick_tock).await })
    };

    let (server, server_handle) = server::Server::run(app_config.server, data)
        .map_err(|e| anyhow::anyhow!("server run: {}", e))?;

    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigterm = signal(SignalKind::terminate())?;
    tokio::select! {
        res = kafka_handle => { match res {
            Ok(_) => {
                info!("kafka task stopped");
                Ok(())
            },
            Err(e)  => Err(anyhow::anyhow!(e)),
        }}

        res = process_handle => { match res {
            Ok(_) => {
                info!("process task stopped");
                Ok(())
            },
            Err(e) => Err(anyhow::anyhow!(e)),
        }}

        res = server_handle => { match res {
            Ok(_) => {
                info!("server task stopped");
                Ok(())
            },
            Err(e) => Err(anyhow::anyhow!(e)),
        }}

        _ = async {
            tokio::select! {
                _ = sigint.recv() => {},
                _ = sigterm.recv() =>{}
            }
        }  => {
            graceful_shutdown(
                SHUTDOWN_INTERVAL_SEC,
                server,
             shutdowner
            ).await
        }
    }
}

async fn graceful_shutdown(
    shutdown_interval_sec: u64,
    server: server::Server,
    shutdowner: Arc<Shutdowner>,
) -> Result<(), anyhow::Error> {
    info!("graceful shutdown started");
    tokio::select! {
        res = async {
            tokio::time::sleep(tokio::time::Duration::from_secs(shutdown_interval_sec)).await;
            Err(anyhow::anyhow!("graceful shutdown failed"))
        } => { res }

        res = async {
            server.stop().await;
            info!("server stopped");

            if shutdowner.stop().await.is_err() {
                return Err(anyhow::anyhow!("tick_sender.send() is failed"));
            };

            shutdowner.finished().await;

            info!("graceful shutdown successfully completed");
            anyhow::Ok(())
        } => { res }
    }
}
