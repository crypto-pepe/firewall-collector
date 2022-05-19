use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub payload_max_size: usize,
}
