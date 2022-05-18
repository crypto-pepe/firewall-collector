use tracing::subscriber::set_global_default;
use tracing_log::LogTracer;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Registry;

pub fn init_tracing() {
    LogTracer::init().expect("failed to set logger");
    let env_filter = EnvFilter::from_default_env();
    let fmt_layer = fmt::Layer::default();
    let subscriber = Registry::default().with(env_filter).with(fmt_layer);
    set_global_default(subscriber).expect("Failed to setup tracing subscriber");
}
