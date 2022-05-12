use deepsize::DeepSizeOf;
use serde::Serialize;
use std::collections::HashMap;

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

    fn add(&mut self, request: Request) {
        self.size += request.deep_size_of();
        self.requests.push(request);
    }

    fn send(&mut self) {
        println!("send");
        self.requests = Vec::new()
    }
}

pub struct Store {
    chunks_by_topics: HashMap<String, Chunk>,
    max_len_chunk: usize,
    max_chunk_size: usize,
}

impl Store {
    pub fn new(max_chunk_size: usize, max_len_chunk: usize) -> Store {
        Store {
            chunks_by_topics: HashMap::new(),
            max_len_chunk,
            max_chunk_size,
        }
    }

    // return true if chunk was sent
    pub fn push(&mut self, request: Request) -> bool {
        let mut is_send = false;
        match self.chunks_by_topics.get_mut(&request.host) {
            Some(n) => {
                if n.is_full(&request) {
                    n.send();
                    is_send = true
                }
                n.add(request.clone());
            }
            None => {
                let mut ch = Chunk::new(self.max_chunk_size, self.max_len_chunk);
                ch.add(request.clone());
                self.chunks_by_topics.insert(request.host, ch);
            }
        };
        is_send
    }

    pub fn send(&mut self) {
        self.chunks_by_topics
            .iter_mut()
            .map(|(_, ch)| ch.send())
            .collect()
    }
}
