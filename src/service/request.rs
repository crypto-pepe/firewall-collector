use std::collections::HashMap;

use actix_web::{web, HttpRequest};
use deepsize::DeepSizeOf;
use serde::Serialize;

use crate::{config::RequestConfig, metrics};

#[derive(Debug, Serialize, DeepSizeOf, Clone, PartialEq)]
pub enum BodyState {
    Original,
    Trimmed,
}

#[derive(Debug, Serialize, DeepSizeOf, Clone, PartialEq)]
pub struct Body {
    pub data: String,
    pub state: BodyState,
}

#[derive(Debug, Serialize, DeepSizeOf, Clone, PartialEq)]
pub struct Request {
    pub timestamp: String,
    pub remote_ip: String,
    pub host: String,
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Body,
}

impl Request {
    pub fn new(
        config: &RequestConfig,
        request: &HttpRequest,
        body: web::Bytes,
    ) -> anyhow::Result<Request> {
        let req = Request {
            timestamp: chrono::offset::Utc::now().to_string(),
            remote_ip: match request
                .headers()
                .iter()
                .find(|(name, _)| name.as_str() == config.ip_header)
            {
                Some((_, value)) => String::from(value.to_str()?),
                None => {
                    return Err(anyhow::anyhow!("ip_header: {} not found", config.ip_header));
                }
            },
            host: match request
                .headers()
                .iter()
                .find(|(name, _)| name.as_str() == config.host_header)
            {
                Some((_, value)) => String::from(value.to_str()?),
                None => {
                    return Err(anyhow::anyhow!(
                        "host_header: {} not found",
                        config.host_header
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
            body: body_handle(body, config.body_max_size)?,
        };

        metrics::HTTP_REQUESTS_TOTAL
            .with_label_values(&[&req.host, &req.method, &req.remote_ip, &req.path])
            .inc();

        Ok(req)
    }
}

fn body_handle(body: web::Bytes, max_size: usize) -> anyhow::Result<Body> {
    let mut state = BodyState::Original;
    let mut b = body.to_vec();
    if b.len() > max_size {
        b = b.as_slice()[..max_size].to_vec();
        state = BodyState::Trimmed;
    }

    Ok(Body {
        data: String::from_utf8(b).map_err(|e| anyhow::anyhow!("failed to body read: {}", e))?,
        state,
    })
}

#[cfg(test)]
mod request_test {
    use actix_web::web;

    use super::{body_handle, Body, BodyState};

    #[test]
    fn body_handle_trim_data_if_too_large() {
        let res = body_handle(web::Bytes::from("some body".as_bytes()), 4);

        assert_eq!(
            res.unwrap(),
            Body {
                data: "some".to_string(),
                state: BodyState::Trimmed
            }
        )
    }
}
