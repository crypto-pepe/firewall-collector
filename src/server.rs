use std::io;

use actix_web::dev::Server;
use actix_web::{web, App, HttpRequest, HttpServer, Responder};
use actix_web_prom::PrometheusMetrics;
use tokio::sync::mpsc;
use tracing::{error, warn};

use crate::config::{self, AppConfig};
use crate::service::{self, Request};

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub sender: mpsc::Sender<service::Request>,
}

pub fn init_server(
    config: config::ServerConfig,
    metrics: PrometheusMetrics,
    data: AppState,
) -> Result<Server, io::Error> {
    Ok(HttpServer::new(move || {
        App::new()
            .wrap(actix_web::middleware::Logger::default())
            .wrap(tracing_actix_web::TracingLogger::default())
            .wrap(metrics.clone())
            .app_data(data.clone())
            .default_service(web::to(default_handler))
    })
    .bind((config.host, config.port))?
    .run())
}

async fn default_handler(
    req: HttpRequest,
    body: web::Bytes,
    state: web::Data<AppState>,
) -> impl Responder {
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
