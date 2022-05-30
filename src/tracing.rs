use std::env;
use tracing::subscriber::set_global_default;
use tracing_log::LogTracer;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::format::Format;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Registry;

pub fn init_tracing() {
    LogTracer::init().expect("failed to set logger");
    let env_filter = EnvFilter::from_default_env();

    let subscriber = Registry::default().with(env_filter);

    let f = Format::default();
    let format = env::var("RUST_LOG_FORMAT").unwrap_or("full".to_string());
    match format.as_str() {
        "json" => {
            let event_format = f.json().flatten_event(true);
            let subscriber = subscriber.with(fmt::Layer::default().event_format(event_format));
            set_global_default(subscriber).expect("Failed to setup tracing subscriber")
        }
        _ => {
            let event_format = f;
            let subscriber = subscriber.with(fmt::Layer::default().event_format(event_format));
            set_global_default(subscriber).expect("Failed to setup tracing subscriber")
        }
    };
}
