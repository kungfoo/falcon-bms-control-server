use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;

use crate::enet_server::EnetServer;
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
mod state;
mod texture_reader;
mod texture_stream;

fn main() {
    let config: Config = Figment::new()
        .merge(Serialized::defaults(Config::default()))
        .merge(Toml::file("config.toml"))
        .extract()
        .expect("Failed to parse config.");

    let cancelled = Arc::new(AtomicBool::new(false));
    let cancel_handle = Arc::clone(&cancelled);

    let env = Env::default().filter_or("LOG_LEVEL", &config.log_level);
    env_logger::init_from_env(env);

    let version = option_env!("VERGEN_GIT_DESCRIBE").unwrap_or("Could not determine version!");

    info!("falcon-bms-control server: {}", version);
    debug!("Config is: {:?}", &config);

    let state = State::new(InnerState::new(cancel_handle));

    let enet_server = EnetServer::new(config.listen_address, config.listen_port, state.clone());

    let handle = thread::spawn(move || {
        enet_server.run();
    });

    handle.join().unwrap();
    info!("Shutting down...");
}
