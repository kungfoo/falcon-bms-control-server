use std::ops::Deref;
use std::sync::Arc;

use std::sync::Mutex;

use std::collections::HashMap;

use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct State {
    inner: Arc<InnerState>,
}

pub struct InnerState {
    pub streams_running: Arc<Mutex<HashMap<StreamKey, CancellationToken>>>,
    pub cancellation_token: CancellationToken,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct StreamKey {
    pub peer: String,
    pub identifier: String,
}

impl State {
    pub fn new(inner: InnerState) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

impl InnerState {
    pub fn new(cancellation_token: CancellationToken) -> Self {
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
