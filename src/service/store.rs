use crate::config::ServiceConfig;
use deepsize::DeepSizeOf;
use std::{collections::HashMap, sync::Mutex};
use thiserror::Error;

use super::{cleaner, Request};

#[derive(Debug)]
struct Chunk {
    requests: Vec<Request>,
    size: usize,
    max_len_chunk: usize,
    max_chunk_size: usize,
}

impl Chunk {
    fn new(max_chunk_size: usize, max_len_chunk: usize) -> Chunk {
        Chunk {
            requests: Vec::new(),
            size: 0,
            max_len_chunk,
            max_chunk_size,
        }
    }

    fn is_full(&self, request: &Request) -> bool {
        let rs = request.deep_size_of();
        if self.size + rs > self.max_chunk_size || self.requests.len() >= self.max_len_chunk {
            return true;
        }
        false
    }

    fn push(&mut self, request: Request) {
        self.size += request.deep_size_of();
        self.requests.push(request);
    }

    fn pop_all(&mut self) -> Vec<Request> {
        let req = self.requests.clone();
        self.requests = Vec::new();
        req
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("internal error: {0}")]
    InternalError(String),
    #[error("reject: '{0}'")]
    Reject(String),
}

#[derive(Debug)]
pub struct Store {
    chunks_by_topics: Mutex<HashMap<String, Chunk>>,

    hosts_to_topics: HashMap<String, String>,

    sensitive_headers: Vec<String>,
    sensitive_json_keys: Vec<String>,

    max_len_chunk: usize,
    max_size_chunk: usize,
}

impl Store {
    pub fn new(config: &ServiceConfig) -> Store {
        Store {
            hosts_to_topics: config.hosts_to_topics.clone(),
            chunks_by_topics: Mutex::new(HashMap::new()),
            sensitive_headers: config.sensitive_headers.clone(),
            sensitive_json_keys: config.sensitive_json_keys.clone(),
            max_len_chunk: config.max_len_chunk,
            max_size_chunk: config.max_size_chunk,
        }
    }

    // return requests if chunk is full
    pub fn push(&self, request: Request) -> Result<Option<(String, Vec<Request>)>, Error> {
        let request = self.clean(request);

        let topic = match self.hosts_to_topics.get(&request.host) {
            Some(t) => t,
            None => {
                return Err(Error::Reject(format!(
                    "host {} not supported",
                    request.host
                )));
            }
        };

        let mut chunks_by_topics = match self.chunks_by_topics.lock() {
            Ok(c) => c,
            Err(e) => {
                return Err(Error::InternalError(format!(
                    "chunks_by_topics.lock(): {}",
                    e
                )));
            }
        };

        match chunks_by_topics.get_mut(topic) {
            Some(chunk) => match chunk.is_full(&request) {
                true => {
                    let reqs = chunk.pop_all();
                    chunk.push(request.clone());
                    Ok(Option::Some((topic.clone(), reqs)))
                }
                false => {
                    chunk.push(request.clone());
                    Ok(Option::None)
                }
            },
            None => {
                let mut ch = Chunk::new(self.max_size_chunk, self.max_len_chunk);
                ch.push(request.clone());
                chunks_by_topics.insert(topic.clone(), ch);
                Ok(Option::None)
            }
        }
    }

    // return all requests from all chunks
    pub fn pop_all(&self) -> Result<Vec<(String, Vec<Request>)>, Error> {
        match self.chunks_by_topics.lock() {
            Ok(mut c) => Ok(c
                .iter_mut()
                .map(|(topic, chunk)| (topic.clone(), chunk.pop_all()))
                .collect()),
            Err(e) => Err(Error::InternalError(e.to_string())),
        }
    }

    fn clean(&self, request: Request) -> Request {
        Request {
            headers: cleaner::headers(request.headers, self.sensitive_headers.clone()),
            body: cleaner::body(request.body, self.sensitive_json_keys.clone()),
            ..request
        }
    }
}

#[cfg(test)]
mod store_test {
    use super::{Request, Store};
    use crate::config::{RequestConfig, ServiceConfig};
    use pepe_config::DurationString;
    use std::{collections::HashMap, time::Duration};

    const HOST: &str = "host1";
    const TOPIC: &str = "topic1";

    #[test]
    fn nothing_return_if_store_is_empty() {
        let store = Store::new(&ServiceConfig {
            ..init_service_config()
        });

        assert!(store.push(Request { ..init_request() }).unwrap().is_none())
    }

    #[test]
    fn return_chunk_by_max_len_chunk() {
        let store = Store::new(&ServiceConfig {
            max_len_chunk: 2,
            ..init_service_config()
        });

        assert!(store.push(Request { ..init_request() }).unwrap().is_none());
        assert!(store.push(Request { ..init_request() }).unwrap().is_none());
        match store.push(Request { ..init_request() }).unwrap() {
            Some((topic, requests)) => {
                assert_eq!(topic, TOPIC.to_string());
                assert_eq!(2, requests.len())
            }
            None => assert!(false),
        };

        // check last request
        assert_eq!(store.pop_all().unwrap()[0].1.len(), 1);
    }

    #[test]
    fn return_chunk_by_max_size_chunk() {
        let store = Store::new(&ServiceConfig {
            max_len_chunk: 10240,
            ..init_service_config()
        });

        assert!(store.push(Request { ..init_request() }).unwrap().is_none());
        assert!(store.push(Request { ..init_request() }).unwrap().is_none());
        match store
            .push(Request {
                body: std::iter::repeat("X").take(1024).collect::<String>(),
                ..init_request()
            })
            .unwrap()
        {
            Some((topic, requests)) => {
                assert_eq!(topic, TOPIC.to_string());
                assert_eq!(2, requests.len())
            }
            None => assert!(false),
        };
    }

    #[test]
    fn pop_all() {
        let store = Store::new(&ServiceConfig {
            hosts_to_topics: HashMap::from([
                (HOST.to_string(), TOPIC.to_string()),
                ("host2".to_string(), "topic2".to_string()),
            ]),
            ..init_service_config()
        });

        assert!(store.push(Request { ..init_request() }).unwrap().is_none());
        assert!(store
            .push(Request {
                host: "host2".to_string(),
                ..init_request()
            })
            .unwrap()
            .is_none());
        assert!(store.push(Request { ..init_request() }).unwrap().is_none());

        assert_eq!(store.pop_all().unwrap().len(), 2);
    }

    fn init_service_config() -> ServiceConfig {
        ServiceConfig {
            request: RequestConfig {
                host_header: "custom_host_header".to_string(),
                ip_header: "ip_header".to_string(),
            },
            max_size_chunk: 2048,
            max_len_chunk: 10,
            sensitive_headers: Vec::from(["some_header".to_string()]),
            sensitive_json_keys: Vec::from(["some_json_key".to_string()]),
            max_collect_chunk_duration: DurationString::new(Duration::new(1, 0)),
            hosts_to_topics: HashMap::from([(HOST.to_string(), TOPIC.to_string())]),
        }
    }

    fn init_request() -> Request {
        Request {
            timestamp: "time".to_string(),
            remote_ip: String::from("0.0.0.0"),
            host: String::from(HOST.to_string()),
            method: String::from("POST"),
            path: String::from("/path"),
            headers: HashMap::from([(String::from("heeader1"), String::from("header value"))]),
            body: String::from("body"),
        }
    }
}
