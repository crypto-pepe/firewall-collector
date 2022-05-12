use crate::config::AppConfig;
use crate::store::Request;
use actix_web::{middleware::Logger, web, App, HttpRequest, HttpServer, Responder};
use env_logger::Env;
use pepe_config::load;
use pepe_log::{error, info, warn};
use tokio::sync::mpsc;

mod config;
mod store;

const DEFAULT_CONFIG: &str = include_str!("../config.yaml");

struct AppState {
    config: AppConfig,
    sender: mpsc::Sender<store::Request>,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    info!("start application");

    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let app_config: AppConfig = match load(DEFAULT_CONFIG, ::config::FileFormat::Yaml) {
        Ok(a) => a,
        Err(e) => panic!("panic {:?}", e),
    };
    info!("config loaded"; "config" => &app_config);

    let (sender, receiver) = mpsc::channel::<store::Request>(32);

    let port = app_config.server.port;

    let data = web::Data::new(AppState {
        config: app_config.clone(),
        sender,
    });

    tokio::spawn(async move { process(receiver, &app_config).await });

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(data.clone())
            .default_service(web::to(index))
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await
}

async fn index(req: HttpRequest, body: web::Bytes, state: web::Data<&AppState>) -> impl Responder {
    if body.len() > state.config.server.payload_max_size {
        warn!("request body is too large");
        return "";
    }

    if let Err(e) = state
        .sender
        .send(Request {
            remote_ip: match req.peer_addr() {
                Some(n) => n.ip().to_string(),
                None => String::new(),
            },
            host: match req.uri().host() {
                Some(n) => String::from(n),
                None => String::new(),
            },
            method: req.method().to_string(),
            path: req.uri().to_string(),
            headers: req
                .headers()
                .iter()
                .map(|(header_name, header_value)| -> (String, String) {
                    (
                        header_name.to_string(),
                        match header_value.to_str() {
                            Ok(s) => String::from(s),
                            Err(_) => String::new(),
                        },
                    )
                })
                .collect(),
            body: String::from_utf8(body.to_vec()).unwrap(),
        })
        .await
    {
        error!("sender: {}", e)
    };

    "OK"
}

async fn process(mut reciver: mpsc::Receiver<store::Request>, config: &AppConfig) {
    let mut store = store::Store::new(config.service.max_size_chunk, config.service.max_len_chunk);

    let mut delay = tokio::time::interval(config.service.max_collect_chunk_duration.into());

    loop {
        tokio::select! {
            Some(msg) = reciver.recv() => {
                if store.push(msg) {
                    delay.reset();
                };
                println!("add")
            }

            _ = delay.tick() => {
                store.send();
                println!("click")
            }
        }
    }
}
