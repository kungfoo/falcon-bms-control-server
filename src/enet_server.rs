use log::{debug, info, trace};
use rmp_serde::decode::Error;
use rusty_enet::{Event, Peer};
use tokio_util::sync::CancellationToken;

use std::net::UdpSocket;

use rusty_enet::HostSettings;

use std::time::Duration;

use crate::msgpack::Message;

pub struct EnetServer {
    addr: String,
    cancellation_token: CancellationToken,
}

impl EnetServer {
    pub fn new(addr: &str, cancellation_token: CancellationToken) -> Self {
        Self {
            addr: addr.to_string(),
            cancellation_token,
        }
    }

    pub async fn run(&self) {
        let socket = UdpSocket::bind(&self.addr).expect("Failed to bind socket.");
        let mut host = rusty_enet::Host::new(socket, HostSettings::default())
            .expect("Failed to setup enet host.");

        loop {
            if self.cancellation_token.is_cancelled() {
                break;
            }

            if let Ok(message) = host.service()
                && let Some(event) = message
            {
                match event {
                    Event::Connect { peer, .. } => {
                        info!("Peer connected: {}", describe_peer(peer));
                    }
                    Event::Disconnect { peer, .. } => {
                        info!("Peer disconnected: {}", describe_peer(peer));
                    }
                    Event::Receive {
                        peer,
                        channel_id,
                        packet,
                    } => {
                        trace!(
                            "Received: peer: {}, channel: {}, data: {:?}",
                            describe_peer(peer),
                            channel_id,
                            packet
                        );

                        let message: Result<Message, Error> = rmp_serde::from_slice(packet.data());
                        match message {
                            Ok(message) => {
                                debug!("msgpack message: {:?}", message)
                            }
                            Err(e) => debug!("failed to parse msgpack message due to: {}", e),
                        }
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(16)).await;
        }
    }
}

fn describe_peer(peer: &Peer<UdpSocket>) -> String {
    if let Some(address) = peer.address() {
        return format!("{}:{}", address.ip(), address.port());
    }
    "Could not determine peer address.".to_string()
}
