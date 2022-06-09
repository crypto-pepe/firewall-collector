use actix_web::http::header;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use core::result::Result::Ok;
use prometheus::{Encoder, TextEncoder};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, warn};

use crate::config::{self, AppConfig};
use crate::service::{self, Request};

use super::error::Error;

pub struct AppState {
    pub config: AppConfig,
    pub sender: mpsc::Sender<(String, Vec<service::Request>)>,
    pub store: Arc<service::Store>,
}

pub struct Server {
    srv_handle: actix_server::ServerHandle,
}

impl Server {
    pub fn run(
        config: config::ServerConfig,
        data: AppState,
    ) -> anyhow::Result<(Server, tokio::task::JoinHandle<Result<(), std::io::Error>>)> {
        let data = web::Data::new(data);
        let srv = match HttpServer::new(move || {
            App::new()
                .wrap(actix_web::middleware::Logger::default())
                .wrap(tracing_actix_web::TracingLogger::default())
                .app_data(data.clone())
                .route("/metrics", web::get().to(metrics))
                .default_service(web::to(default_handler))
        })
        .disable_signals()
        .bind((config.host, config.port))
        {
            Ok(s) => s.run(),
            Err(e) => return Err(anyhow::anyhow!("{}", e)),
        };

        let s = Server {
            srv_handle: srv.handle(),
        };
        let server_handle = tokio::spawn(async move { srv.await });
        Ok((s, server_handle))
    }

    pub async fn stop(&self) {
        self.srv_handle.stop(true).await;
    }
}

async fn default_handler(
    req: HttpRequest,
    body: web::Bytes,
    state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let req = match Request::new(&state.config.service.request, &req, body) {
        Ok(r) => r,
        Err(e) => {
            warn!("failed to parse request: {}", e);
            return Ok(HttpResponse::NoContent().finish());
        }
    };

    match state.store.push(req) {
        Ok(s) => match s {
            Some((topic, requests)) => match state.sender.send((topic.clone(), requests)).await {
                Ok(_) => Ok(HttpResponse::Created().finish()),
                Err(e) => {
                    error!("{}", e);
                    Err(Error::InternalError)
                }
            },
            None => Ok(HttpResponse::Created().finish()),
        },
        Err(e) => match e {
            service::Error::InternalError(e) => {
                error!("{}", e);
                Err(Error::InternalError)
            }
            service::Error::Reject(e) => {
                warn!("{}", e);
                Ok(HttpResponse::NoContent().finish())
            }
        },
    }
}

pub async fn metrics() -> Result<HttpResponse, Error> {
    let encoder = TextEncoder::new();
    let mut buffer = vec![];
    encoder
        .encode(&prometheus::gather(), &mut buffer)
        .expect("Failed to encode metrics");

    let response = String::from_utf8(buffer.clone()).expect("Failed to convert bytes to string");
    buffer.clear();

    Ok(HttpResponse::Ok()
        .insert_header(header::ContentType(mime::TEXT_PLAIN))
        .body(response))
}
