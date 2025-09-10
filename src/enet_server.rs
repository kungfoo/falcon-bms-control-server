use log::{debug, error, info, trace};
use rmp_serde::decode::Error;
use rusty_enet::{Event, Peer};
use tokio_util::sync::CancellationToken;

use std::net::UdpSocket;

use rusty_enet::HostSettings;

use std::time::Duration;

use crate::{
    State, StreamKey,
    msgpack::{Command, Message},
};

pub struct EnetServer {
    addr: String,
    state: State,
}

impl EnetServer {
    pub fn new(addr: &str, state: State) -> Self {
        Self {
            addr: addr.to_string(),
            state,
        }
    }

    pub async fn run(&self) {
        let socket = UdpSocket::bind(&self.addr).expect("Failed to bind socket.");
        let mut host = rusty_enet::Host::new(socket, HostSettings::default())
            .expect("Failed to setup enet host.");

        loop {
            if self.state.cancellation_token.is_cancelled() {
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
                            Ok(message) => self.handle_message(peer, message).await,
                            Err(e) => debug!("failed to parse msgpack message due to: {}", e),
                        }
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(16)).await;
        }
    }

    async fn handle_message(&self, peer: &Peer<UdpSocket>, message: Message) {
        match message {
            Message::IcpButtonPressed { .. } => {}
            Message::IcpButtonReleased { .. } => {}
            Message::OsbButtonPressed { .. } => {}
            Message::OsbButtonReleased { .. } => {}
            Message::StreamedTextureRequest {
                identifier,
                command,
                ..
            } => match command {
                Command::Start => {
                    let token = CancellationToken::new();
                    let key = StreamKey {
                        peer: describe_peer(peer),
                        identifier,
                    };

                    let mut streams = self.state.streams_running.lock().unwrap();
                    streams.insert(key, token);
                    debug!("streams running: {:?}", streams.len());
                }
                Command::Stop => {
                    let key = StreamKey {
                        peer: describe_peer(peer),
                        identifier,
                    };
                    let mut streams = self.state.streams_running.lock().unwrap();
                    streams.get(&key).map(|token| token.cancel());
                    streams.remove(&key);

                    debug!("streams running: {:?}", streams.len());
                }
            },
            msg => {
                error!("Received unexpected message via enet: {:?}", msg)
            }
        }
    }
}

fn describe_peer(peer: &Peer<UdpSocket>) -> String {
    if let Some(address) = peer.address() {
        return format!("{}:{}", address.ip(), address.port());
    }
    "Could not determine peer address.".to_string()
}
