use crate::config::ServiceConfig;
use deepsize::DeepSizeOf;
use pepe_log::warn;
use std::collections::HashMap;

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

#[derive(Debug)]
pub struct Store {
    hosts_to_topics: HashMap<String, String>,
    chunks_by_topics: HashMap<String, Chunk>,

    sensitive_headers: Vec<String>,
    sensitive_json_keys: Vec<String>,

    max_len_chunk: usize,
    max_size_chunk: usize,
}

impl Store {
    pub fn new(config: &ServiceConfig) -> Store {
        Store {
            hosts_to_topics: config.hosts_to_topics.clone(),
            chunks_by_topics: HashMap::new(),
            sensitive_headers: config.sensitive_headers.clone(),
            sensitive_json_keys: config.sensitive_json_keys.clone(),
            max_len_chunk: config.max_len_chunk,
            max_size_chunk: config.max_size_chunk,
        }
    }

    // return requests if chunk is full
    pub fn push(&mut self, request: Request) -> Option<(String, Vec<Request>)> {
        let request = self.clean(request);

        match self.hosts_to_topics.get(&request.host) {
            Some(topic) => match self.chunks_by_topics.get_mut(topic) {
                Some(chunk) => {
                    if chunk.is_full(&request) {
                        let reqs = chunk.pop_all();
                        chunk.push(request.clone());
                        return Option::Some((topic.clone(), reqs));
                    }
                    chunk.push(request.clone());
                }
                None => {
                    let mut ch = Chunk::new(self.max_size_chunk, self.max_len_chunk);
                    ch.push(request.clone());
                    self.chunks_by_topics.insert(topic.clone(), ch);
                }
            },
            None => {
                warn!("host {} not supported", request.host)
            }
        };
        Option::None
    }

    // return all requests from all chunks
    pub fn pop_all(&mut self) -> Vec<(String, Vec<Request>)> {
        return self
            .chunks_by_topics
            .iter_mut()
            .map(|(topic, chunk)| (topic.clone(), chunk.pop_all()))
            .collect();
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
    use crate::config::ServiceConfig;
    use duration_string::DurationString;
    use std::{collections::HashMap, time::Duration};

    const HOST: &str = "host1";
    const TOPIC: &str = "topic1";

    #[test]
    fn nothing_return_if_store_is_empty() {
        let mut store = Store::new(&ServiceConfig {
            ..init_service_config()
        });

        assert!(store.push(Request { ..init_request() }).is_none())
    }

    #[test]
    fn return_chunk_by_max_len_chunk() {
        let mut store = Store::new(&ServiceConfig {
            max_len_chunk: 2,
            ..init_service_config()
        });

        assert!(store.push(Request { ..init_request() }).is_none());
        assert!(store.push(Request { ..init_request() }).is_none());
        match store.push(Request { ..init_request() }) {
            Some((topic, requests)) => {
                assert_eq!(topic, TOPIC.to_string());
                assert_eq!(2, requests.len())
            }
            None => assert!(false),
        };

        // check last request
        assert_eq!(store.pop_all()[0].1.len(), 1);
    }

    #[test]
    fn return_chunk_by_max_size_chunk() {
        let mut store = Store::new(&ServiceConfig {
            max_len_chunk: 10240,
            ..init_service_config()
        });

        assert!(store.push(Request { ..init_request() }).is_none());
        assert!(store.push(Request { ..init_request() }).is_none());
        match store.push(Request {
            body: std::iter::repeat("X").take(1024).collect::<String>(),
            ..init_request()
        }) {
            Some((topic, requests)) => {
                assert_eq!(topic, TOPIC.to_string());
                assert_eq!(2, requests.len())
            }
            None => assert!(false),
        };
    }

    #[test]
    fn pop_all() {
        let mut store = Store::new(&ServiceConfig {
            hosts_to_topics: HashMap::from([
                (HOST.to_string(), TOPIC.to_string()),
                ("host2".to_string(), "topic2".to_string()),
            ]),
            ..init_service_config()
        });

        assert!(store.push(Request { ..init_request() }).is_none());
        assert!(store
            .push(Request {
                host: "host2".to_string(),
                ..init_request()
            })
            .is_none());
        assert!(store.push(Request { ..init_request() }).is_none());

        assert_eq!(store.pop_all().len(), 2);
    }

    fn init_service_config() -> ServiceConfig {
        ServiceConfig {
            max_size_chunk: 2048,
            max_len_chunk: 10,
            sensitive_headers: Vec::from(["some_header".to_string()]),
            sensitive_json_keys: Vec::from(["some_json_key".to_string()]),
            max_collect_chunk_duration: DurationString::new(Duration::new(1, 0)),
            hosts_to_topics: HashMap::from([(HOST.to_string(), TOPIC.to_string())]),
            kafka_brokers: Vec::new(),
        }
    }

    fn init_request() -> Request {
        Request {
            remote_ip: String::from("0.0.0.0"),
            host: String::from(HOST.to_string()),
            method: String::from("POST"),
            path: String::from("/path"),
            headers: HashMap::from([(String::from("heeader1"), String::from("header value"))]),
            body: String::from("body"),
        }
    }
}
