use std::{net::UdpSocket, time::Duration};

use crate::{State, msgpack::ProtocolMessage};
use log::{debug, error, info};

pub struct UdpBroadcastListener {
    state: State,
    socket: UdpSocket,
}

impl UdpBroadcastListener {
    pub fn new(address: String, port: u16, state: State) -> Self {
        let socket = UdpSocket::bind(format!("{}:{}", address, port)).unwrap();
        socket
            .set_nonblocking(true)
            .expect("Failed to set socket to non-blocking");
        info!("Listening for broadcast packets on {}:{}", address, port);
        Self { state, socket }
    }

    pub fn run(&mut self) {
        let &mut UdpBroadcastListener {
            state: _,
            ref socket,
        } = self;

        let mut buf = [0u8; 1024];

        loop {
            if self
                .state
                .cancellation_token
                .load(std::sync::atomic::Ordering::Relaxed)
            {
                break;
            }

            let recv = socket.recv_from(&mut buf);
            match recv {
                Ok((size, addr)) => {
                    let message: Result<ProtocolMessage, rmp_serde::decode::Error> =
                        rmp_serde::from_slice(&buf[..size]);
                    match message {
                        Ok(message) => match message {
                            ProtocolMessage::Hello {} => {
                                debug!("Received packet from {}", addr);
                                let message = ProtocolMessage::Ack {};
                                let bytes = rmp_serde::to_vec_named(&message)
                                    .expect("Failed to serialize ack message");
                                socket
                                    .send_to(&bytes, addr)
                                    .expect("Failed to send ack message to peer");
                            }
                            m => error!("Received unexpected message via udp: {:?}", m),
                        },
                        Err(e) => error!("Failed to parse message received via UDP: {}", e),
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    error!("UDP error: {}", e);
                }
            }
        }
    }
}
