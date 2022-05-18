use std::collections::HashMap;

use actix_web::{web, App, HttpRequest, HttpServer, Responder};
use actix_web_prom::PrometheusMetricsBuilder;
use pepe_config::load;
use tokio::sync::mpsc;
use tracing::subscriber::set_global_default;
use tracing::{error, info, warn};
use tracing_log::LogTracer;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Registry;

use crate::config::AppConfig;
use crate::service::Request;

mod config;
mod kafka;
mod service;

const DEFAULT_CONFIG: &str = include_str!("../config.yaml");

struct AppState {
    config: AppConfig,
    sender: mpsc::Sender<service::Request>,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    LogTracer::init().expect("failed to set logger");
    info!("start application");

    let env_filter = EnvFilter::from_default_env();
    let fmt_layer = fmt::Layer::default();
    let subscriber = Registry::default().with(env_filter).with(fmt_layer);
    set_global_default(subscriber).expect("Failed to setup tracing subscriber");

    let app_config: AppConfig = match load(DEFAULT_CONFIG, ::config::FileFormat::Yaml) {
        Ok(a) => a,
        Err(e) => panic!("panic {:?}", e),
    };
    info!(
        "config loaded:\n{:?}",
        match serde_json::to_string_pretty(&app_config) {
            Ok(s) => s,
            Err(e) => panic!("config: {}", e),
        }
    );

    let port = app_config.server.port;

    let (kafka_sender, kafka_receiver) = mpsc::channel::<(String, Vec<service::Request>)>(32);
    let config_process = app_config.clone();
    tokio::spawn(async move {
        if let Err(e) = kafka::producer(kafka_receiver, config_process).await {
            error!("kafka producer: {}", e)
        }
    });

    let (request_sender, request_receiver) = mpsc::channel::<service::Request>(32);
    let config_pocess = app_config.service.clone();
    tokio::spawn(
        async move { service::process(request_receiver, kafka_sender, config_pocess).await },
    );

    let data = web::Data::new(AppState {
        config: app_config.clone(),
        sender: request_sender,
    });

    let prometheus = PrometheusMetricsBuilder::new("api")
        .endpoint("/metrics")
        .const_labels(HashMap::from([(
            "service".to_string(),
            "firewall-collector".to_string(),
        )]))
        .build()
        .unwrap();

    HttpServer::new(move || {
        App::new()
            .wrap(actix_web::middleware::Logger::default())
            .wrap(tracing_actix_web::TracingLogger::default())
            .wrap(prometheus.clone())
            .app_data(data.clone())
            .default_service(web::to(index))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}

async fn index(req: HttpRequest, body: web::Bytes, state: web::Data<AppState>) -> impl Responder {
    if body.len() > state.config.server.payload_max_size {
        warn!("request body is too large");
        return "";
    }

    let r = Request {
        remote_ip: match req.peer_addr() {
            Some(n) => n.ip().to_string(),
            None => String::new(),
        },
        host: match req.uri().host() {
            Some(n) => String::from(n),
            None => String::new(),
        },
        method: req.method().to_string(),
        path: req.uri().to_string(),
        headers: req
            .headers()
            .iter()
            .map(|(header_name, header_value)| -> (String, String) {
                (
                    header_name.to_string(),
                    match header_value.to_str() {
                        Ok(s) => String::from(s),
                        Err(_) => String::new(),
                    },
                )
            })
            .collect(),
        body: String::from_utf8(body.to_vec()).unwrap(),
    };

    if let Err(e) = state.sender.send(r).await {
        error!("sender: {}", e)
    };

    "OK"
}
