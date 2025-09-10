use log::{debug, error};
use std::time::Duration;
use tokio::sync::mpsc::Sender;

use rusty_enet::Packet;
use tokio_util::sync::CancellationToken;

use crate::{packet_shuttle::PacketShuttleMessage, state::StreamKey};

pub struct TextureStream {
    cancellation_token: CancellationToken,
    stream_key: StreamKey,
    tx: Sender<PacketShuttleMessage>,
}

impl TextureStream {
    pub fn new(
        cancellation_token: CancellationToken,
        stream_key: StreamKey,
        tx: Sender<PacketShuttleMessage>,
    ) -> Self {
        Self {
            cancellation_token,
            stream_key,
            tx,
        }
    }

    pub async fn run(&self) {
        loop {
            if self.cancellation_token.is_cancelled() {
                debug!("Cancelled streaming {:?}", self.stream_key);
                break;
            }

            tokio::time::sleep(Duration::from_millis(33)).await;
            let message = format!("Hello: {:?}", self.stream_key);
            let packet = Packet::unreliable_unsequenced(message.as_bytes());

            let result = self
                .tx
                .send(PacketShuttleMessage {
                    peer_id: self.stream_key.peer_id,
                    channel: 0,
                    packet,
                })
                .await;
            match result {
                Ok(_) => {}
                Err(e) => error!("Failed to send shuttle message: {}", e),
            }
        }
    }
}
