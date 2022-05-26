# firewall-collector

The service collects HTTP requests into chunks by hosts and sending them to kafka topics.

## Requirements

The topics from parameter `service.hosts_to_topics` of the config have to be created in the Kafka.

## Configuration

### Environment variables

| Name        | Required | Note                                                                     |
| ----------- | -------- | ------------------------------------------------------------------------ |
| RUST_LOG    | No       | Log level. https://docs.rs/env_logger/0.9.0/env_logger/#enabling-logging |
| CONFIG_PATH | No       | Path to the `yaml` formatted config file                                 |

### Config variables

If CONFIG_PATH is not stated then ./config.yaml will be used

| Name                               | Required | Note                                                                                             |
| ---------------------------------- | -------- | ------------------------------------------------------------------------------------------------ |
| server.host                        | No       | Socket host to bind. Default `0.0.0.0`.                                                          |
| server.port                        | No       | Socket port to bind. Default `8080`.                                                             |
| server.payload_max_size            | Yes      | Maxumum size HTTP body in bytes.                                                                 |
| service.host_header                | Yes      | Header name for extracting `host` from the request. Should be lower-cased.                       |
| service.max_len_chunk              | Yes      | Maximum number requests in the chunk.                                                            |
| service.max_size_chunk             | Yes      | Maximum size chunk in bytes.                                                                     |
| service.max_collect_chunk_duration | Yes      | How long does accumulate chunk.                                                                  |
| service.hosts_to_topics            | Yes      | Hosts to Kafka topics mapping.                                                                   |
| service.sensitive_headers          | No       | Headers that will be excluded from requests.                                                     |
| service.sensitive_json_keys        | No       | JSON keys that will be excluded from request body.If body isn't json, body exclueded completely. |
| kafka.brokers                      | Yes      | Kafka brokers hosts.                                                                             |
| kafka.ack_timeout                  | No       | Time the kafka brokers await the receipt of acknowledgements. Default 1 sec.                     |

---

Each of the configuration parameter can be overridden via the environment variable. Nested values overriding are supported via the '.' separator.

Example:

| Parameter name | Env. variable |
| -------------- | ------------- |
| some_field     | SOME_FIELD    |
| server.port    | SERVER.PORT   |
