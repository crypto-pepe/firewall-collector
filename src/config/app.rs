use super::server::ServerConfig;
use super::service::ServiceConfig;
use serde::{Deserialize, Serialize};
use slog_extlog_derive::SlogValue;

#[derive(Debug, Clone, Deserialize, Serialize, SlogValue)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub service: ServiceConfig,
}
