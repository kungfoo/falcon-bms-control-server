use crate::texture_reader::TextureReader;
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
use turbojpeg::{Compressor, Image, PixelFormat, Subsamp};

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
        let texture_id: TextureId = self.stream_key.identifier.as_str().into();
        let mut reader = TextureReader::new();

        let mut compressor = match Compressor::new() {
            Ok(mut c) => {
                if let Err(e) = c.set_quality(self.stream_options.quality.into()) {
                    warn!(
                        "Failed to set JPEG quality for {:?}: {}",
                        self.stream_key, e
                    );
                }
                // DED is pure thin orange text on black, so 4:4:4 keeps glyphs crisp;
                // 4:2:1 gives sharper green/cyan cockpit text than 4:2:0 on MFDs & RWR.
                let subsamp = match texture_id {
                    TextureId::Ded => Subsamp::None,
                    _ => Subsamp::Sub2x1,
                };
                if let Err(e) = c.set_subsamp(subsamp) {
                    warn!(
                        "Failed to set JPEG subsamp for {:?}: {}",
                        self.stream_key, e
                    );
                }
                c
            }
            Err(e) => {
                error!(
                    "Failed to initialize turbojpeg Compressor for {:?}: {}",
                    self.stream_key, e
                );
                return;
            }
        };

        let mut jpeg_buf: Vec<u8> = Vec::with_capacity(64 * 1024);
        let mut settled = true;
        let mut last_read_error: Option<String> = None;

        loop {
            if self.cancellation_token.load(Ordering::Relaxed) {
                debug!("Cancelled streaming {:?}", self.stream_key);
                break;
            }

            thread::sleep(Duration::from_millis(
                1000 / self.stream_options.refresh_rate.max(1) as u64,
            ));

            let data = reader.read(texture_id.clone());

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
                        // Frame is unchanged. If the last send was unreliable, resend the
                        // already-compressed bytes once reliably so a dropped UDP fragment
                        // never leaves the client stuck on a stale static screen.
                        if !settled && !jpeg_buf.is_empty() {
                            let packet_data = PacketData {
                                peer_id: self.stream_key.peer_id,
                                data: jpeg_buf.clone(),
                                channel: texture_id.clone() as u8,
                                reliable: true,
                            };
                            if let Err(e) = self.tx.send(packet_data) {
                                error!(
                                    "Failed to send settle packet for {:?}, stopping stream: {}",
                                    self.stream_key, e
                                );
                                break;
                            }
                            settled = true;
                        }
                        continue;
                    }

                    let tj_image = Image {
                        pixels: image.as_raw().as_slice(),
                        width: image.width() as usize,
                        pitch: image.width() as usize * 3, // 3 bytes per pixel (RGB)
                        height: image.height() as usize,
                        format: PixelFormat::RGB,
                    };

                    match compressor.compress_to_vec(tj_image) {
                        Ok(bytes) => {
                            jpeg_buf = bytes;
                            let packet_data = PacketData {
                                peer_id: self.stream_key.peer_id,
                                data: jpeg_buf.clone(),
                                channel: texture_id.clone() as u8,
                                reliable: false,
                            };

                            if let Err(e) = self.tx.send(packet_data) {
                                error!(
                                    "Failed to send packet_data for {:?}, stopping stream: {}",
                                    self.stream_key, e
                                );
                                break;
                            } else {
                                self.last_hash.replace(hash);
                                settled = false;
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
