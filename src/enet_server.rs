use crate::{
    msgpack::{Command, Message},
    state::{State, StreamKey},
    texture_stream,
};
use enet::{Address, Enet, Host, Packet};
use log::{debug, error, info, trace};
use uuid::Uuid;

use std::{
    net::Ipv4Addr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::Duration,
};

pub struct EnetServer {
    address: String,
    port: u16,
    state: State,
}

pub struct WrappedHost {
    host: Arc<Mutex<Host<PeerData>>>,
    rx: Receiver<PacketData>,
}

#[derive(Clone, Debug)]
pub struct PacketData {
    pub peer_id: String,
    pub data: Vec<u8>,
    pub channel: u8,
}

impl WrappedHost {
    pub fn new(host: Host<PeerData>, rx: Receiver<PacketData>) -> Self {
        WrappedHost {
            host: Arc::new(Mutex::new(host)),
            rx,
        }
    }

    pub fn queue_packets_to_send(&self) {
        // TODO: Maybe queue multiples
        let to_send = self.rx.recv_timeout(Duration::from_millis(5));
        if let Ok(to_send) = to_send {
            let mut host = self
                .host
                .lock()
                .expect("Could not lock host to send packet");
            let peer = host.peers().find(|peer| {
                let p = peer.data().map(|data| data.id.clone());
                if let Some(p) = p {
                    return p == to_send.peer_id;
                }
                false
            });
            if let Some(mut peer) = peer {
                let result = peer.send_packet(
                    Packet::new(&to_send.data, enet::PacketMode::UnreliableSequenced)
                        .expect("Failed to create enet packet"),
                    to_send.channel,
                );

                match result {
                    Ok(_) => trace!("Queued packet!"),
                    Err(e) => error!("Failed to send packet because of: {}", e),
                }
            }
        }
    }
}

unsafe impl Send for WrappedHost {}
unsafe impl Send for PeerData {}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct PeerData {
    pub id: String,
}

impl EnetServer {
    pub fn new(address: String, port: u16, state: State) -> Self {
        Self {
            address,
            port,
            state,
        }
    }

    pub fn run(&self) {
        let ipv4 = self.address.parse::<Ipv4Addr>().unwrap();
        let address = Address::new(ipv4, self.port);
        let enet = Enet::new().unwrap();
        let host: Host<PeerData> = enet
            .create_host(
                Some(&address),
                32,
                enet::ChannelLimit::Limited(10),
                enet::BandwidthLimit::Unlimited,
                enet::BandwidthLimit::Unlimited,
            )
            .unwrap();

        debug!("Host address: {:?}", host.address());

        let (tx, rx) = mpsc::channel();

        let wrapped_host = WrappedHost::new(host, rx);

        loop {
            if self.state.cancellation_token.load(Ordering::Relaxed) {
                break;
            }

            wrapped_host.queue_packets_to_send();

            let mut locked_host = wrapped_host.host.lock().unwrap();
            let service_result = locked_host.service(100).unwrap();
            if let Some(mut event) = service_result {
                match event {
                    enet::Event::Connect(ref mut peer) => {
                        peer.set_data(Some(PeerData {
                            id: Uuid::new_v4().to_string(),
                        }));
                        info!("Peer connected: {:?}", peer);
                    }
                    enet::Event::Disconnect(ref peer, _) => {
                        info!("Peer disconnected: {:?}", peer);
                        self.state
                            .cancel_all_streams(peer.data().map(|d| d.id.clone()));
                    }
                    enet::Event::Receive {
                        ref sender,
                        channel_id,
                        ref packet,
                    } => {
                        let payload = packet.data();
                        let message: Result<Message, rmp_serde::decode::Error> =
                            rmp_serde::from_slice(payload);
                        let peer_id = sender
                            .data()
                            .expect("Sender did not have data attached")
                            .id
                            .clone();
                        match message {
                            Ok(message) => {
                                self.handle_message(tx.clone(), peer_id, channel_id, message)
                            }
                            Err(e) => {
                                error!("Failed to parse message due to: {}", e);
                            }
                        }
                    }
                }
            }

            let _ = thread::sleep(Duration::from_millis(10));
        }

        info!("Shutting down enet server");
    }

    fn handle_message(
        &self,
        tx: Sender<PacketData>,
        peer_id: String,
        _channel_id: u8,
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
                    let token = Arc::new(AtomicBool::new(false));
                    let key = StreamKey {
                        peer_id,
                        identifier,
                    };

                    let mut streams = self.state.streams_running.lock().unwrap();
                    streams.insert(key.clone(), token.clone());
                    debug!("starting: {:?}", key);

                    let texture_stream = texture_stream::TextureStream::new(token, key, tx);

                    let _ = thread::spawn(move || {
                        texture_stream.run();
                    });
                }
                Command::Stop => {
                    let key = StreamKey {
                        peer_id,
                        identifier,
                    };
                    let mut streams = self.state.streams_running.lock().unwrap();
                    if let Some(token) = streams.get(&key) {
                        token.store(true, Ordering::Relaxed);
                    }
                    streams.remove(&key);

                    debug!("stopping: {:?}", key);
                }
            },
            msg => {
                error!("Received unexpected message via enet: {:?}", msg)
            }
        }
    }
}
