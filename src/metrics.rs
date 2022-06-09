use lazy_static::lazy_static;
use prometheus::{opts, register_int_counter_vec, IntCounterVec};

lazy_static! {
    pub static ref HTTP_REQUESTS_TOTAL: IntCounterVec = register_int_counter_vec!(
        opts!("http_requests_total", "HTTP requests total"),
        &["host", "method", "ip", "path"]
    )
    .expect("Can't create a metric");
}
