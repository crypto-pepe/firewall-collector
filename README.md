# firewall-collector

The service collects HTTP requests into chunks by hosts and sending them to kafka topics.

## Configuration

Example:

```yaml
server:
  port: 8080
  payload_max_size: 51200
service:
  max_len_chunk: 2
  max_size_chunk: 1024
  max_collect_chunk_duration: 3s
  hosts_by_topics:
    topic1:
      - "host1"
      - "host2"
    topic2:
      - "host3"
  kafka_brokers:
    - "localhost:9092"
```

- `payload_max_size`: maxumum size HTTP body in bytes
- `max_len_chunk`: maximum number requests in the chunk
- `max_size_chunk`: maximum size chunk in bytes
- `max_collect_chunk_duration`: how long does accumulate chunk
- `hosts_by_topics`: ratio hosts to Kafka topics
