use super::server::ServerConfig;
use super::service::ServiceConfig;
use anyhow::Result;
use pepe_config::load;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub service: ServiceConfig,
}

const DEFAULT_CONFIG: &str = include_str!("../../config.yaml");

impl AppConfig {
    pub fn load() -> Result<AppConfig> {
        load(DEFAULT_CONFIG, ::config::FileFormat::Yaml).map_err(|e| e.into())
    }
}
