use actix_web::{middleware::Logger, web, App, HttpRequest, HttpServer, Responder};
use env_logger::Env;
use pepe_config::load;
use pepe_log::{error, info, warn};
use tokio::sync::mpsc;

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
    info!("start application");

    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let app_config: AppConfig = match load(DEFAULT_CONFIG, ::config::FileFormat::Yaml) {
        Ok(a) => a,
        Err(e) => panic!("panic {:?}", e),
    };
    info!("config loaded"; "config" => &app_config);

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

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
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
