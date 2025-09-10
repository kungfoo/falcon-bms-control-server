use crate::texture_reader;
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

use crate::{enet_server::PacketData, state::StreamKey, texture_reader::TextureId};

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

            thread::sleep(Duration::from_millis(
                1000 / self.stream_options.refresh_rate as u64,
            ));

            let texture_id: TextureId = self.stream_key.identifier.as_str().into();
            let data = texture_reader::rtt_texture_read(texture_id.clone());

            if let Ok(image) = data {
                // make it a jpeg as requested

                let bytes = turbojpeg::compress(
                    image.as_deref(),
                    self.stream_options.quality.into(),
                    turbojpeg::Subsamp::None,
                );

                let bytes = bytes.expect("Failed to encode jpeg");

                let packet_data = PacketData {
                    peer_id: self.stream_key.peer_id.clone(),
                    data: bytes.to_vec(),
                    channel: texture_id as u8,
                };

                if let Err(e) = self.tx.send(packet_data) {
                    error!("Failed to send packet_data: {}", e)
                }
            } else {
                // TODO: for now this is okay
                // error!("Failed to read texture data for: {:?}", texture_id)
            }
        }
    }
}
