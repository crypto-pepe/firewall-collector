use std::collections::HashMap;

use deepsize::DeepSizeOf;
use serde::Serialize;

#[derive(Debug, Serialize, DeepSizeOf, Clone, PartialEq)]
pub struct Request {
    pub remote_ip: String,
    pub host: String,
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}
