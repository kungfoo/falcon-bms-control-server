use crate::{
    packet_shuttle::{PacketShuttle, PacketShuttleMessage},
    state::{State, StreamKey},
    texture_stream,
};
use log::{debug, error, info, trace};
use rmp_serde::decode::Error;
use rusty_enet::{Event, Peer};
use tokio::sync::mpsc::Sender;
use tokio::sync::{Mutex, mpsc};
use tokio_util::sync::CancellationToken;

use std::{net::UdpSocket, sync::Arc};

use rusty_enet::HostSettings;

use std::time::Duration;

use crate::msgpack::{Command, Message};

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
        let enet_host = rusty_enet::Host::new(socket, HostSettings::default())
            .expect("Failed to setup enet host.");

        let host = Arc::new(Mutex::new(enet_host));

        let (tx, rx) = mpsc::channel(256);
        let mut packet_shuttle = PacketShuttle::new(host.clone(), rx);
        tokio::spawn(async move {
            packet_shuttle.run().await;
        });

        loop {
            if self.state.cancellation_token.is_cancelled() {
                break;
            }

            if let Ok(message) = host.lock().await.service()
                && let Some(event) = message
            {
                match event {
                    Event::Connect { peer, .. } => {
                        info!("Peer connected: {}", describe_peer(peer));
                    }
                    Event::Disconnect { peer, .. } => {
                        info!("Peer disconnected: {}", describe_peer(peer));
                        self.state.cancel_all_streams(peer.id());
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
                            Ok(message) => self.handle_message(tx.clone(), peer, message).await,
                            Err(e) => debug!("failed to parse msgpack message due to: {}", e),
                        }
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }

    async fn handle_message(
        &self,
        tx: Sender<PacketShuttleMessage>,
        peer: &Peer<UdpSocket>,
        message: Message,
    ) {
        match message {
            Message::IcpButtonPressed { icp, button } => debug!("{}:{}", icp, button),
            Message::IcpButtonReleased { icp, button } => debug!("{}:{}", icp, button),
            Message::OsbButtonPressed { mfd, osb } => debug!("{}:{}", mfd, osb),
            Message::OsbButtonReleased { mfd, osb } => debug!("{}:{}", mfd, osb),
            Message::StreamedTextureRequest {
                identifier,
                command,
                ..
            } => match command {
                Command::Start => {
                    let token = CancellationToken::new();
                    let key = StreamKey {
                        peer_id: peer.id(),
                        identifier,
                    };

                    let mut streams = self.state.streams_running.lock().unwrap();
                    streams.insert(key.clone(), token.clone());
                    debug!("streams running: {:?}", streams.len());

                    let texture_stream = texture_stream::TextureStream::new(token, key, tx.clone());
                    tokio::spawn(async move {
                        texture_stream.run().await;
                    });
                }
                Command::Stop => {
                    let key = StreamKey {
                        peer_id: peer.id(),
                        identifier,
                    };
                    let mut streams = self.state.streams_running.lock().unwrap();
                    if let Some(token) = streams.get(&key) {
                        token.cancel()
                    }
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
