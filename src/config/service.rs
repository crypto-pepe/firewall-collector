use duration_string::DurationString;
use serde::{Deserialize, Serialize};
use slog_extlog_derive::SlogValue;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize, SlogValue)]
pub struct ServiceConfig {
    pub max_size_chunk: usize,
    pub max_len_chunk: usize,
    pub max_collect_chunk_duration: DurationString,
    pub hosts_by_topics: HashMap<String, Vec<String>>,
}
