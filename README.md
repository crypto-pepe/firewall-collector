# firewall-collector

The service collects HTTP requests into chunks by hosts and sending them to kafka topics.

## Configuration

### Environment variables

| Name        | Required | Note                                                                     |
| ----------- | -------- | ------------------------------------------------------------------------ |
| RUST_LOG    | NO       | Log level. https://docs.rs/env_logger/0.9.0/env_logger/#enabling-logging |
| CONFIG_PATH | No       | Path to the `yaml` formatted config file                                 |

YAML formatted config file example:

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
      - 'host1'
      - 'host2'
    topic2:
      - 'host3'
  kafka_brokers:
    - 'localhost:9092'
```

- `payload_max_size`: maxumum size HTTP body in bytes
- `max_len_chunk`: maximum number requests in the chunk
- `max_size_chunk`: maximum size chunk in bytes
- `max_collect_chunk_duration`: how long does accumulate chunk
- `hosts_by_topics`: hosts to Kafka topics mapping

## Requirements

The topics from parameter `service.hosts_by_topics` of the config have to be created in the Kafka.
