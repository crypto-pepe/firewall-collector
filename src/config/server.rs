use serde::{Deserialize, Serialize};
use slog_extlog_derive::SlogValue;

#[derive(Debug, Clone, Deserialize, Serialize, SlogValue)]
pub struct ServerConfig {
    pub port: u16,
    pub payload_max_size: usize,
}
