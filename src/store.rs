use crate::config::AppConfig;
use deepsize::DeepSizeOf;
use pepe_log::{error, warn};
use serde::Serialize;
use std::collections::HashMap;
use tokio::sync::mpsc;

#[derive(Debug, Serialize, DeepSizeOf, Clone)]
pub struct Request {
    pub remote_ip: String,
    pub host: String,
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

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

pub struct Store {
    relation_hosts_to_topics: HashMap<String, String>,
    chunks_by_topics: HashMap<String, Chunk>,
    kafka_sender: mpsc::Sender<(String, Vec<Request>)>,

    max_len_chunk: usize,
    max_size_chunk: usize,
}

impl Store {
    pub fn new(config: &AppConfig, kafka_sender: mpsc::Sender<(String, Vec<Request>)>) -> Store {
        let relation_hosts_to_topics = config
            .service
            .hosts_by_topics
            .iter()
            .flat_map(|(topic, hosts)| {
                hosts
                    .iter()
                    .map(|host| (host.to_string(), topic.to_string()))
            })
            .collect();

        Store {
            relation_hosts_to_topics,
            chunks_by_topics: HashMap::new(),
            kafka_sender,
            max_len_chunk: config.service.max_len_chunk,
            max_size_chunk: config.service.max_size_chunk,
        }
    }

    // return true if chunk was sent
    pub async fn push(&mut self, request: Request) -> bool {
        let mut is_send = false;
        match self.relation_hosts_to_topics.get(&request.host) {
            Some(topic) => match self.chunks_by_topics.get_mut(topic) {
                Some(chunk) => {
                    if chunk.is_full(&request) {
                        if let Err(e) = self
                            .kafka_sender
                            .send((topic.clone(), chunk.pop_all()))
                            .await
                        {
                            error!("sender: {}", e)
                        }
                        is_send = true
                    }
                    chunk.push(request.clone());
                }
                None => {
                    let mut ch = Chunk::new(self.max_size_chunk, self.max_len_chunk);
                    ch.push(request.clone());
                    self.chunks_by_topics.insert(request.host, ch);
                }
            },
            None => {
                warn!("host {} not supported", request.host)
            }
        };
        is_send
    }

    pub async fn send(&mut self) {
        for (topic, chunk) in &mut self.chunks_by_topics {
            let reqs = chunk.pop_all();
            if let Err(e) = self.kafka_sender.send((topic.to_string(), reqs)).await {
                error!("sender: {}", e)
            }
        }
    }
}
