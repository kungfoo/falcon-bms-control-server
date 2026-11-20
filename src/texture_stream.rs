use crate::texture_reader;
use log::{debug, error, info, warn};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
    },
    thread,
    time::Duration,
};
use turbojpeg::Image;
use turbojpeg::PixelFormat;

use crate::{enet_server::PacketData, state::StreamKey, texture_reader::TextureId};

pub struct TextureStream {
    cancellation_token: Arc<AtomicBool>,
    stream_key: StreamKey,
    stream_options: StreamOptions,
    tx: Sender<PacketData>,
    last_hash: Option<u64>,
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
            last_hash: None,
        }
    }

    pub fn stream_key(&self) -> &StreamKey {
        &self.stream_key
    }

    pub fn run(&mut self) {
        let mut last_read_error: Option<String> = None;

        loop {
            if self.cancellation_token.load(Ordering::Relaxed) {
                debug!("Cancelled streaming {:?}", self.stream_key);
                break;
            }

            thread::sleep(Duration::from_millis(
                1000 / self.stream_options.refresh_rate.max(1) as u64,
            ));

            let texture_id: TextureId = self.stream_key.identifier.as_str().into();
            let data = texture_reader::rtt_texture_read(texture_id.clone());

            match data {
                Ok(image) => {
                    if last_read_error.take().is_some() {
                        info!(
                            "Recovered shared memory texture read for {:?}",
                            self.stream_key
                        );
                    }

                    let hash = seahash::hash(image.as_raw());

                    if let Some(last_hash) = self.last_hash
                        && last_hash == hash
                    {
                        // the last sent frame is the same as this one.
                        continue;
                    }

                    let tj_image = Image {
                        pixels: image.as_raw().as_slice(),
                        width: image.width() as usize,
                        pitch: image.width() as usize * 3, // 3 bytes per pixel (RGB)
                        height: image.height() as usize,
                        format: PixelFormat::RGB,
                    };

                    // make it a jpeg as requested
                    let bytes = turbojpeg::compress(
                        tj_image,
                        self.stream_options.quality.into(),
                        turbojpeg::Subsamp::Sub2x2,
                    );

                    match bytes {
                        Ok(bytes) => {
                            let packet_data = PacketData {
                                peer_id: self.stream_key.peer_id,
                                data: bytes.to_vec(),
                                channel: texture_id as u8,
                            };

                            if let Err(e) = self.tx.send(packet_data) {
                                error!(
                                    "Failed to send packet_data for {:?}, stopping stream: {}",
                                    self.stream_key, e
                                );
                                break;
                            } else {
                                self.last_hash.replace(hash);
                            }
                        }
                        Err(e) => {
                            warn!(
                                "Failed to compress JPEG for {:?} ({}x{}): {}",
                                self.stream_key,
                                image.width(),
                                image.height(),
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    let msg = e.to_string();
                    if last_read_error.as_deref() != Some(&msg) {
                        warn!(
                            "Failed to read texture data for {:?}: {}",
                            self.stream_key, msg
                        );
                        last_read_error = Some(msg);
                    }
                }
            }
        }
    }
}
