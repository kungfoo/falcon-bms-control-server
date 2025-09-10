use log::error;
use rusty_enet::error::PeerSendError;
use std::{net::UdpSocket, sync::Arc};

use rusty_enet::{Host, Packet, PeerID};
use tokio::sync::Mutex;

pub struct PacketShuttleMessage {
    pub peer_id: PeerID,
    pub channel: u8,
    pub packet: Packet,
}

/// Apparently enet does not like being bombarded by multiple tokio runnables/threads
/// Let's put an mpsc channel between them.
pub struct PacketShuttle {
    host: Arc<Mutex<Host<UdpSocket>>>,
    rx: tokio::sync::mpsc::Receiver<PacketShuttleMessage>,
}

impl PacketShuttle {
    pub fn new(
        host: Arc<Mutex<Host<UdpSocket>>>,
        rx: tokio::sync::mpsc::Receiver<PacketShuttleMessage>,
    ) -> Self {
        Self { host, rx }
    }

    pub async fn run(&mut self) {
        while let Some(message) = self.rx.recv().await {
            let mut host = self.host.lock().await;
            let peer = host.peer_mut(message.peer_id);
            let sent = peer.send(message.channel, &message.packet);
            match sent {
                Ok(_) => {
                    // intentionally left blank
                }
                Err(e) => match e {
                    PeerSendError::NotConnected => {
                        error!("Failed to send to unconnected peer: {:?}", message.peer_id)
                    }
                    PeerSendError::InvalidChannel => error!("Invalid channel used to send data"),
                    PeerSendError::PacketTooLarge => error!("Packet too large to send"),
                    PeerSendError::FragmentsExceeded => error!("Fragments exceeded"),
                    PeerSendError::FailedToQueue => error!("Failed to queue packet"),
                },
            }
        }
    }
}
