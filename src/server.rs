use actix_web::{dev, web, App, HttpRequest, HttpResponse, HttpServer};
use actix_web_prom::PrometheusMetrics;
use anyhow::Result;
use core::result::Result::Ok;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::config::{self, AppConfig};
use crate::service::{self, Request};

pub struct AppState {
    pub config: AppConfig,
    pub sender: mpsc::Sender<(String, Vec<service::Request>)>,
    pub store: Arc<service::Store>,
}

pub fn init_server(
    config: config::ServerConfig,
    metrics: PrometheusMetrics,
    data: AppState,
) -> Result<dev::Server> {
    let data = web::Data::new(data);
    match HttpServer::new(move || {
        App::new()
            .wrap(actix_web::middleware::Logger::default())
            .wrap(tracing_actix_web::TracingLogger::default())
            .wrap(metrics.clone())
            .app_data(data.clone())
            .default_service(web::to(default_handler))
    })
    .bind((config.host, config.port))
    {
        Ok(s) => Ok(s.run()),
        Err(e) => Err(anyhow::anyhow!("{}", e)),
    }
}

async fn default_handler(
    req: HttpRequest,
    body: web::Bytes,
    state: web::Data<AppState>,
) -> HttpResponse {
    if body.len() > state.config.server.payload_max_size {
        return HttpResponse::NoContent().body("request body is too large");
    }

    let req = Request {
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

    match state.store.push(req) {
        Ok(s) => {
            if let Some((topic, requests)) = s {
                if let Err(e) = state.sender.send((topic.clone(), requests)).await {
                    HttpResponse::InternalServerError().body(e.to_string());
                }
            }
            HttpResponse::Created().body("")
        }
        Err(e) => HttpResponse::NoContent()
            .reason(Box::leak(e.to_string().into_boxed_str()))
            .body(""),
    }
}
