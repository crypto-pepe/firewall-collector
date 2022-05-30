use std::collections::HashMap;

use actix_web::{web, HttpRequest};
use deepsize::DeepSizeOf;
use serde::Serialize;

use crate::config::RequestConfig;

#[derive(Debug, Serialize, DeepSizeOf, Clone, PartialEq, valuable::Valuable)]
pub struct Request {
    pub timestamp: String,
    pub remote_ip: String,
    pub host: String,
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl Request {
    pub fn new(
        request_config: &RequestConfig,
        request: &HttpRequest,
        body: web::Bytes,
    ) -> anyhow::Result<Request> {
        let req = Request {
            timestamp: chrono::offset::Utc::now().to_string(),
            remote_ip: match request
                .headers()
                .iter()
                .find(|(name, _)| name.as_str() == request_config.ip_header)
            {
                Some((_, value)) => String::from(value.to_str()?),
                None => {
                    return Err(anyhow::anyhow!(
                        "ip_header: {} not found",
                        request_config.ip_header
                    ));
                }
            },
            host: match request
                .headers()
                .iter()
                .find(|(name, _)| name.as_str() == request_config.host_header)
            {
                Some((_, value)) => String::from(value.to_str()?),
                None => {
                    return Err(anyhow::anyhow!(
                        "host_header: {} not found",
                        request_config.host_header
                    ));
                }
            },
            method: request.method().to_string(),
            path: request.uri().to_string(),
            headers: request
                .headers()
                .iter()
                .map(|(header_name, header_value)| -> (String, String) {
                    (
                        header_name.to_string(),
                        match header_value.to_str() {
                            Ok(s) => String::from(s),
                            Err(_) => String::new(),
                        },
                    )
                })
                .collect(),
            body: match String::from_utf8(body.to_vec()) {
                Ok(s) => s,
                Err(e) => return Err(anyhow::anyhow!("{}", e.to_string())),
            },
        };

        Ok(req)
    }
}
