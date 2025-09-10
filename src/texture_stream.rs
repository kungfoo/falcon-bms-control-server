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
    stream_options: StreamOptions,
    tx: Sender<PacketData>,
}

#[derive(Debug)]
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

impl TextureStream {
    pub fn new(
        cancellation_token: Arc<AtomicBool>,
        stream_key: StreamKey,
        stream_options: StreamOptions,
        tx: Sender<PacketData>,
    ) -> Self {
        Self {
            cancellation_token,
            stream_key,
            stream_options,
            tx,
        }
    }

    pub fn run(&self) {
        loop {
            if self.cancellation_token.load(Ordering::Relaxed) {
                debug!("Cancelled streaming {:?}", self.stream_key);
                break;
            }

            thread::sleep(Duration::from_millis(16));
            let message = format!("Hello: {:?}:{:?}", self.stream_key, self.stream_options);

            let packet_data = PacketData {
                peer_id: self.stream_key.peer_id.clone(),
                data: message.into_bytes(),
                channel: 0,
            };

            if let Err(e) = self.tx.send(packet_data) {
                error!("Failed to send packet_data: {}", e)
            }
        }
    }
}
