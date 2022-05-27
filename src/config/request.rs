use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RequestConfig {
    pub host_header: String,
    pub ip_header: String,
}
