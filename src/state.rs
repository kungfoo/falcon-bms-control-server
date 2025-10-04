use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use std::sync::Mutex;

use std::collections::HashSet;

use enet::PeerID;

#[derive(Clone)]
pub struct State {
    inner: Arc<InnerState>,
}

pub struct InnerState {
    pub streams_running: Arc<Mutex<HashSet<StreamKey>>>,
    pub cancellation_token: Arc<AtomicBool>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct StreamKey {
    pub peer_id: PeerID,
    pub identifier: String,
    pub stream_options: StreamOptions,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct StreamOptions {
    pub refresh_rate: u16,
    pub quality: u16,
}

impl StreamOptions {
    pub fn new(refresh_rate: Option<u16>, quality: Option<u16>) -> Self {
        Self {
            refresh_rate: refresh_rate.unwrap_or(30),
            quality: quality.unwrap_or(65),
        }
    }
}

impl State {
    pub fn new(inner: InnerState) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }

    pub fn cancel_all_streams(&self, id: PeerID) {
        let mut streams = self.streams_running.lock().unwrap();
        let keys_to_cancel: Vec<StreamKey> = streams
            .iter()
            .filter(|key| key.peer_id == id)
            .cloned()
            .collect();

        for key in keys_to_cancel {
            streams.remove(&key);
        }
    }

    pub fn streams_to_send(&self) -> Vec<StreamKey> {
        let streams = self.streams_running.lock().unwrap();
        streams.iter().cloned().collect()
    }

    pub fn start_stream(&self, key: StreamKey) {
        let mut streams = self.streams_running.lock().unwrap();
        streams.insert(key);
    }

    pub fn stop_stream(&self, key: StreamKey) {
        let mut streams = self.streams_running.lock().unwrap();
        streams.remove(&key);
    }
}

impl InnerState {
    pub fn new(cancellation_token: Arc<AtomicBool>) -> Self {
        Self {
            streams_running: Arc::new(Mutex::new(HashSet::new())),
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
