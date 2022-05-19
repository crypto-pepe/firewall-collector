use std::{ collections::HashMap};

use actix_web_prom::{PrometheusMetrics, PrometheusMetricsBuilder};

pub fn init_api_metrics() -> PrometheusMetrics {
    match PrometheusMetricsBuilder::new("api")
        .endpoint("/metrics")
        .const_labels(HashMap::from([(
            "service".to_string(),
            "firewall-collector".to_string(),
        )]))
        .build()
    {
        Ok(r) => r,
        Err(e) => panic!("mitrics: {}", e),
    }
}
