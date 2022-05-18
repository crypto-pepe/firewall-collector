use std::io::{Error, ErrorKind};

use super::server::ServerConfig;
use super::service::ServiceConfig;
use pepe_config::load;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub service: ServiceConfig,
}

const DEFAULT_CONFIG: &str = include_str!("../../config.yaml");

impl AppConfig {
    pub fn load() -> Result<AppConfig, Error> {
        match load(DEFAULT_CONFIG, ::config::FileFormat::Yaml) {
            Ok(c) => Ok(c),
            Err(e) => Err(Error::new(ErrorKind::Other, e.to_string())),
        }
    }
}
