use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use std::sync::Mutex;

use std::collections::HashMap;

use enet::PeerID;

#[derive(Clone)]
pub struct State {
    inner: Arc<InnerState>,
}

pub struct InnerState {
    pub streams_running: Arc<Mutex<HashMap<StreamKey, Arc<AtomicBool>>>>,
    pub cancellation_token: Arc<AtomicBool>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct StreamKey {
    pub peer_id: PeerID,
    pub identifier: String,
}

impl State {
    pub fn new(inner: InnerState) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }

    pub fn cancel_all_streams(&self, id: PeerID) {
        let streams = self.streams_running.lock().unwrap();
        let keys_to_cancel = streams.keys().filter(|key| key.peer_id == id);
        for key in keys_to_cancel {
            streams.get(key).unwrap().store(true, Ordering::Relaxed);
        }
    }
}

impl InnerState {
    pub fn new(cancellation_token: Arc<AtomicBool>) -> Self {
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
