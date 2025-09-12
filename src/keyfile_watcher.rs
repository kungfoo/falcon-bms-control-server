use std::{sync::mpsc::Sender, thread, time::Duration};

use log::debug;

use crate::state::State;

pub enum KeyfileMessage {}

pub struct KeyfileWatcher {
    tx: Sender<KeyfileMessage>,
    state: State,
}

impl KeyfileWatcher {
    pub fn new(tx: Sender<KeyfileMessage>, state: State) -> Self {
        Self { tx, state }
    }

    pub fn run(&self) {
        loop {
            if self
                .state
                .cancellation_token
                .load(std::sync::atomic::Ordering::Relaxed)
            {
                debug!("Cancelling...");
                break;
            }

            debug!("checking for new keyfile...");

            thread::sleep(Duration::from_secs(5));
        }
    }
}
