use pepe_config::DurationString;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceConfig {
    pub host_header: String,
    pub max_size_chunk: usize,
    pub max_len_chunk: usize,
    pub max_collect_chunk_duration: DurationString,
    pub hosts_to_topics: HashMap<String, String>,
    pub sensitive_headers: Vec<String>,
    pub sensitive_json_keys: Vec<String>,
}
