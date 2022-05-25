use std::collections::HashMap;

use actix_web::{web, HttpRequest};
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

impl Request {
    pub fn new(request: HttpRequest, body: web::Bytes) -> anyhow::Result<Request> {
        let req = Request {
            remote_ip: match request.peer_addr() {
                Some(n) => n.ip().to_string(),
                None => String::new(),
            },
            host: match request
                .headers()
                .iter()
                .find(|(name, _)| name.as_str() == "host")
            {
                Some((_, value)) => String::from(value.to_str()?),
                None => String::from("127.0.0.1"),
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
