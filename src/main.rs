use crate::state::InnerState;
use crate::state::State;
use config::Config;

use env_logger::Env;
use figment::Figment;
use figment::providers::Format;
use figment::providers::Serialized;
use figment::providers::Toml;
use log::debug;
use log::info;
use serde::Serialize;

mod config;
mod enet_server;
mod msgpack;
mod packet_shuttle;
mod state;
mod texture_stream;

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
