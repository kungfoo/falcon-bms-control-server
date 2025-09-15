use crate::keyfile_watcher::KeyfileWatcher;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;

use crate::enet_server::EnetServer;
use crate::state::InnerState;
use crate::state::State;
use callbacks::CallbackSender;
use config::Config;

use env_logger::Env;
use figment::Figment;
use figment::providers::Format;
use figment::providers::Serialized;
use figment::providers::Toml;
use log::debug;
use log::info;
use messages::Message;

mod callbacks;
mod config;
mod enet_server;
mod keyboard_emulator;
mod keyfile_watcher;
mod messages;
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

    // comms channels for threads
    let (tx, rx) = std::sync::mpsc::channel::<Message>();

    let state = State::new(InnerState::new(cancel_handle));

    // all the stuff we're running concurrently
    let enet_server = EnetServer::new(
        tx.clone(),
        config.listen_address,
        config.listen_port,
        state.clone(),
    );
    let mut key_filewatcher = KeyfileWatcher::new(tx.clone(), state.clone());
    let mut callback_sender = CallbackSender::new(rx, state.clone());

    // run all of them
    let h1 = thread::spawn(move || enet_server.run());
    let h2 = thread::spawn(move || key_filewatcher.run());
    let h3 = thread::spawn(move || callback_sender.run());

    let _ = h1.join();
    let _ = h2.join();
    let _ = h3.join();
    info!("Shutting down...");
}
