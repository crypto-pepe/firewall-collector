use super::server::ServerConfig;
use super::service::ServiceConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub service: ServiceConfig,
}
