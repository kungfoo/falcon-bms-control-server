use std::collections::HashMap;
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;
use std::sync::Mutex;

use env_logger::Env;
use figment::Figment;
use figment::providers::Format;
use figment::providers::Serialized;
use figment::providers::Toml;
use log::debug;
use log::info;
use serde::Deserialize;
use serde::Serialize;
use tokio_util::sync::CancellationToken;

mod enet_server;
mod msgpack;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    log_level: String,
    listen_address: String,
    listen_port: u16,
    broadcast_port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            listen_address: "0.0.0.0".to_string(),
            listen_port: 9022,
            broadcast_port: 9020,
        }
    }
}

#[derive(Clone)]
struct State {
    inner: Arc<InnerState>,
}

struct InnerState {
    streams_running: Arc<Mutex<HashMap<StreamKey, CancellationToken>>>,
    cancellation_token: CancellationToken,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct StreamKey {
    peer: String,
    identifier: String,
}

impl State {
    fn new(inner: InnerState) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

impl InnerState {
    fn new(cancellation_token: CancellationToken) -> Self {
        Self {
            streams_running: Arc::new(Mutex::new(HashMap::new())),
            cancellation_token,
        }
    }
}

impl Deref for State {
    type Target = InnerState;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[tokio::main]
async fn main() {
    let config: Config = Figment::new()
        .merge(Serialized::defaults(Config::default()))
        .merge(Toml::file("config.toml"))
        .extract()
        .expect("Failed to parse config.");

    let env = Env::default().filter_or("LOG_LEVEL", &config.log_level);
    env_logger::init_from_env(env);

    let version = option_env!("VERGEN_GIT_DESCRIBE").unwrap_or("Could not determine version!");

    info!("falcon-bms-control server: {}", version);
    debug!("Config is: {:?}", &config);

    let cancellation_token = tokio_util::sync::CancellationToken::new();

    let state = State::new(InnerState::new(cancellation_token.clone()));

    let addr = format!("{}:{}", config.listen_address, config.listen_port);
    let enet_server = enet_server::EnetServer::new(&addr, state.clone());
    tokio::spawn(async move {
        enet_server.run().await;
    });

    let _ = tokio::signal::ctrl_c().await;
    info!("Shutting down...");
    cancellation_token.cancel();
}
