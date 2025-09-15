use crate::{
    messages::Message,
    msgpack::{Command, ProtocolMessage},
    state::{State, StreamKey},
    texture_stream::{self, StreamOptions},
};
use enet::{Address, Enet, Host, Packet, Peer, PeerID};
use log::{debug, error, info, trace};

use std::{
    net::Ipv4Addr,
    rc::Rc,
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
    callback_tx: Sender<Message>,
}

pub struct WrappedHost {
    host: Rc<Mutex<Host<PeerData>>>,
    rx: Receiver<PacketData>,
}

#[derive(Clone, Debug)]
pub struct PacketData {
    pub peer_id: PeerID,
    pub data: Vec<u8>,
    pub channel: u8,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct PeerData {}

impl WrappedHost {
    pub fn new(host: Host<PeerData>, rx: Receiver<PacketData>) -> Self {
        WrappedHost {
            host: Rc::new(Mutex::new(host)),
            rx,
        }
    }

    /// enet is fighting really hard against being used from more than one thread, and that's okay
    /// so here is code to shuttle packets before calling host.service() again.
    pub fn queue_packets_to_send(&self) {
        loop {
            let to_send = self.rx.recv_timeout(Duration::from_millis(1));
            if let Ok(to_send) = to_send {
                let mut host = self
                    .host
                    .lock()
                    .expect("Could not lock host to send packet");
                if let Some(peer) = host.peer_mut(to_send.peer_id) {
                    let packet = Packet::new(
                        to_send.data,
                        enet::PacketMode::UnreliableUnsequencedUnreliablyFragmented,
                    )
                    .expect("Failed to create enet packet");

                    let result = peer.send_packet(packet, to_send.channel);

                    match result {
                        Ok(_) => trace!("Queued packet!"),
                        Err(e) => error!("Failed to send packet because of: {}", e),
                    }
                }
            } else {
                break;
            }
        }
    }
}

impl EnetServer {
    pub fn new(tx: Sender<Message>, address: String, port: u16, state: State) -> Self {
        Self {
            address,
            port,
            state,
            callback_tx: tx,
        }
    }

    pub fn run(&self) {
        let ipv4 = self.address.parse::<Ipv4Addr>().unwrap();
        let address = Address::new(ipv4, self.port);
        let enet = Enet::new().expect("Failed to setup enet");
        let host: Host<PeerData> = enet
            .create_host(
                Some(&address),
                32,
                enet::ChannelLimit::Limited(10),
                enet::BandwidthLimit::Unlimited,
                enet::BandwidthLimit::Unlimited,
            )
            .expect("Failed to create enet host");

        debug!("Host address: {:?}", host.address());

        let (tx, rx) = mpsc::channel();

        let wrapped_host = WrappedHost::new(host, rx);

        loop {
            if self.state.cancellation_token.load(Ordering::Relaxed) {
                break;
            }

            wrapped_host.queue_packets_to_send();

            let mut locked_host = wrapped_host.host.lock().unwrap();

            let service_result = locked_host.service(Duration::from_millis(0));
            match service_result {
                Ok(service_result) => {
                    if let Some(mut event) = service_result {
                        match event.kind() {
                            enet::EventKind::Connect => {
                                info!("Peer connected: {:?}", event.peer_id());
                                let peer: &mut Peer<PeerData> = event.peer_mut();
                                peer.set_ping_interval(Duration::from_millis(200));
                            }
                            enet::EventKind::Disconnect { data: _ } => {
                                info!("Peer disconnected: {:?}", event.peer_id());
                                self.state.cancel_all_streams(event.peer_id());
                            }
                            enet::EventKind::Receive { channel_id, packet } => {
                                let payload = packet.data();
                                let message: Result<ProtocolMessage, rmp_serde::decode::Error> =
                                    rmp_serde::from_slice(payload);
                                match message {
                                    Ok(message) => self.handle_message(
                                        tx.clone(),
                                        event.peer_id(),
                                        *channel_id,
                                        message,
                                    ),
                                    Err(e) => {
                                        error!("Failed to parse message due to: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => error!("Failed to service host: {}", e),
            }

            thread::sleep(Duration::from_millis(1));
        }

        info!("Shutting down enet server");
    }

    fn handle_message(
        &self,
        tx: Sender<PacketData>,
        peer_id: PeerID,
        _channel_id: u8,
        message: ProtocolMessage,
    ) {
        match message {
            ProtocolMessage::StreamedTextureRequest {
                identifier,
                command,
                refresh_rate,
                quality,
            } => match command {
                Command::Start => {
                    let token = Arc::new(AtomicBool::new(false));
                    let key = StreamKey {
                        peer_id,
                        identifier,
                    };

                    let mut streams = self.state.streams_running.lock().unwrap();
                    streams.insert(key.clone(), token.clone());
                    let stream_options = StreamOptions::new(refresh_rate, quality);
                    debug!("starting: {:?}:{:?}", key, stream_options);
                    let mut texture_stream =
                        texture_stream::TextureStream::new(token, key, stream_options, tx);

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
                debug!("Sending message to callback tx: {:?}", msg);
                let result = self
                    .callback_tx
                    .send(Message::EnetReceived { message: msg });
                if let Err(e) = result {
                    error!("Failed to send callback message: {}", e);
                }
            }
        }
    }
}
