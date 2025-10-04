use crate::{
    state::{State, StreamOptions},
    texture_reader,
};
use enet::PeerID;
use log::{debug, error};
use lru_mem::LruCache;
use std::{
    collections::HashMap,
    sync::{atomic::Ordering, mpsc::Sender},
    thread,
    time::Duration,
};
use turbojpeg::{Image, PixelFormat};

use crate::{enet_server::PacketData, texture_reader::TextureId};

pub struct TextureStream {
    state: State,
    tx: Sender<PacketData>,
    last_encoded: LruCache<LruCacheKey, Vec<u8>>,
    last_sent: LastSent,
}

/// caches last encoded payloads per options and hash,
/// so we can save encoding x times for x peers wanting the same data.
#[derive(Debug, Hash, PartialEq, Eq)]
pub struct LruCacheKey {
    pub stream_options: StreamOptions,
    pub payload_hash: u64,
}

/// stores the last hash sent for a texture identifier per peer.
struct LastSent {
    last_sent: HashMap<LastSentKey, u64>,
}

impl LastSent {
    pub fn new() -> Self {
        Self {
            last_sent: HashMap::new(),
        }
    }

    fn for_peer(&self, peer: PeerID, identifier: String) -> u64 {
        self.last_sent
            .get(&LastSentKey { peer, identifier })
            .cloned()
            .unwrap_or_default()
    }

    fn remember(&mut self, peer: PeerID, identifier: String, data_hash: u64) {
        self.last_sent
            .insert(LastSentKey { peer, identifier }, data_hash);
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct LastSentKey {
    peer: PeerID,
    identifier: String,
}

// let's use 2mb of LRU cache for now.
const MAX_SIZE: usize = 2048 * 1024;

impl TextureStream {
    pub fn new(state: State, tx: Sender<PacketData>) -> Self {
        Self {
            state,
            tx,
            last_encoded: LruCache::new(MAX_SIZE),
            last_sent: LastSent::new(),
        }
    }

    pub fn run(&mut self) {
        loop {
            if self.state.cancellation_token.load(Ordering::Relaxed) {
                debug!("Stopping all streams.");
                break;
            }

            // for now, let's just aim for 60fps
            thread::sleep(Duration::from_millis(16));

            let to_send = self.state.streams_to_send();

            for key in to_send {
                let texture_id: TextureId = key.identifier.as_str().into();
                let data = texture_reader::rtt_texture_read(texture_id.clone());
                let channel = texture_id as u8;

                if let Ok(image) = data {
                    let hash = seahash::hash(image.as_raw());

                    let lru_key = LruCacheKey {
                        stream_options: key.stream_options,
                        payload_hash: hash,
                    };

                    let last_encoded = self.last_encoded.get(&lru_key).cloned();

                    if let Some(encoded) = last_encoded {
                        if self.last_sent.for_peer(key.peer_id, key.identifier.clone()) == hash {
                            // no need to send again.
                            continue;
                        }

                        self.send(key.peer_id, key.identifier.clone(), encoded, channel, hash);
                    } else {
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
                            key.stream_options.quality.into(),
                            turbojpeg::Subsamp::Sub2x2,
                        );

                        let bytes = bytes.expect("Failed to encode jpeg");
                        // TODO: store in LRU cache

                        self.send(key.peer_id, key.identifier, bytes.to_vec(), channel, hash);
                    }
                } else {
                    // TODO: for now this is okay
                    // error!("Failed to read texture data for: {:?}", texture_id)
                }
            }
        }
    }

    fn send(
        &mut self,
        peer_id: PeerID,
        identifier: String,
        bytes: Vec<u8>,
        channel: u8,
        data_hash: u64,
    ) {
        let packet_data = PacketData {
            peer_id,
            data: bytes,
            channel,
        };

        if let Err(e) = self.tx.send(packet_data) {
            error!("Failed to send packet_data: {}", e)
        } else {
            // remember that we sent this.
            self.last_sent.remember(peer_id, identifier, data_hash);
        }
    }
}
