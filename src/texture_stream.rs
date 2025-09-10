use log::{debug, error};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
    },
    thread,
    time::Duration,
};

use crate::{enet_server::PacketData, state::StreamKey};

pub struct TextureStream {
    cancellation_token: Arc<AtomicBool>,
    stream_key: StreamKey,
    tx: Sender<PacketData>,
}

impl TextureStream {
    pub fn new(
        cancellation_token: Arc<AtomicBool>,
        stream_key: StreamKey,
        tx: Sender<PacketData>,
    ) -> Self {
        Self {
            cancellation_token,
            stream_key,
            tx,
        }
    }

    pub fn run(&self) {
        loop {
            if self.cancellation_token.load(Ordering::Relaxed) {
                debug!("Cancelled streaming {:?}", self.stream_key);
                break;
            }

            thread::sleep(Duration::from_millis(33));
            let message = format!("Hello: {:?}", self.stream_key);

            match self.tx.send(PacketData {
                peer_id: self.stream_key.peer_id.clone(),
                data: message.into_bytes(),
                channel: 0,
            }) {
                Ok(_) => {}
                Err(e) => error!("Failed to send packet_data: {}", e),
            }
        }
    }
}
