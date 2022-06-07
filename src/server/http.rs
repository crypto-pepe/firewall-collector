use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use actix_web_prom::PrometheusMetrics;
use core::result::Result::Ok;
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
        metrics: PrometheusMetrics,
        data: AppState,
    ) -> anyhow::Result<(Server, tokio::task::JoinHandle<Result<(), std::io::Error>>)> {
        let data = web::Data::new(data);
        let srv = match HttpServer::new(move || {
            App::new()
                .wrap(actix_web::middleware::Logger::default())
                .wrap(tracing_actix_web::TracingLogger::default())
                .wrap(metrics.clone())
                .app_data(data.clone())
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
    if body.len() > state.config.server.payload_max_size {
        warn!("request body too large");
        return Ok(HttpResponse::NoContent().finish());
    }

    let req = match Request::new(&state.config.service.request, &req, body) {
        Ok(r) => r,
        Err(e) => {
            warn!("failed parse request: {}", e);
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
