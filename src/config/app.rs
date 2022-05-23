use super::server::ServerConfig;
use super::service::ServiceConfig;
use anyhow::Result;
use pepe_config::{kafka, load, FileFormat};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub service: ServiceConfig,
    pub kafka: kafka::Config,
}

const DEFAULT_CONFIG: &str = include_str!("../../config.yaml");

impl AppConfig {
    pub fn load() -> Result<AppConfig> {
        load(DEFAULT_CONFIG, FileFormat::Yaml).map_err(|e| e.into())
    }
}
