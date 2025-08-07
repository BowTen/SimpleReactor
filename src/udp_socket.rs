use std::{
    net::SocketAddr,
    sync::{Arc, atomic::AtomicBool},
};

use log::error;
use mio::Interest;

use crate::{
    SocketRemote, callbacks::DatagramCallback, reactor::ReactorSignal, reactor_channel::Sender,
};

pub struct UdpSocket {
    socket: mio::net::UdpSocket,
    buffer: [u8; 65536],
    datagram_callback: DatagramCallback,
    signal_sender: Sender<ReactorSignal<Self>>,
    remote: Option<Arc<SocketRemote<Self>>>,
    poll_token: Option<mio::Token>,
    pub is_established: Arc<AtomicBool>,
}

impl UdpSocket {
    pub fn new(
        socket: mio::net::UdpSocket,
        datagram_callback: DatagramCallback,
        signal_sender: Sender<ReactorSignal<Self>>,
    ) -> Self {
        UdpSocket {
            socket,
            buffer: [0; 65536],
            datagram_callback,
            signal_sender,
            remote: None,
            poll_token: None,
            is_established: Arc::new(AtomicBool::new(false)),
        }
    }

    // must call after register
    pub fn remote(&self) -> &Arc<SocketRemote<Self>> {
        self.remote
            .as_ref()
            .expect("must call register before accessing remote")
    }
}

impl crate::ReactorSocket for UdpSocket {
    type Socket = mio::net::UdpSocket;
    fn handle_establish(&self, is_established: bool) {
        self.is_established
            .store(is_established, std::sync::atomic::Ordering::Relaxed);
    }

    fn handle_event(&mut self, event: &mio::event::Event, receive_time: std::time::Instant) {
        if event.is_readable() {
            loop {
                match self.socket.recv_from(&mut self.buffer) {
                    Ok((bytes_read, peer_addr)) => (self.datagram_callback)(
                        self.remote().clone(),
                        &mut self.buffer[..bytes_read],
                        peer_addr,
                        receive_time,
                    ),
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        break;
                    }
                    Err(e) => {
                        error!("Error receiving UDP packet: {}", e);
                        self.remote().shutdown();
                        return;
                    }
                }
            }
        }
    }

    fn interest(&self) -> mio::Interest {
        Interest::READABLE
    }

    fn set_interest(&mut self, _interest: mio::Interest) {
        // UDP sockets are always interested in reading
    }

    fn poll_token(&self) -> Option<mio::Token> {
        self.poll_token
    }

    fn set_poll_token(&mut self, token: mio::Token) {
        self.poll_token = Some(token);
        self.remote = Some(Arc::new(SocketRemote::new(
            self.socket.local_addr().unwrap(),
            SocketAddr::from(([0, 0, 0, 0], 0)),
            token,
            self.signal_sender.clone(),
            self.is_established.clone(),
        )));
    }

    fn socket(&mut self) -> &mut Self::Socket {
        &mut self.socket
    }

    fn stash_output(&mut self, _data: &[u8]) {
        // send block, drop data
        panic!("UDP sockets do not support stashing output");
    }

    fn write(&mut self, _data: &[u8]) -> std::io::Result<usize> {
        // UDP sockets do not support writing in the same way as TCP
        panic!("Invalid operation: use send() for UDP sockets");
    }

    fn send(&mut self, addr: std::net::SocketAddr, data: &[u8]) -> std::io::Result<usize> {
        self.socket.send_to(data, addr)
    }

    fn is_established(&self) -> bool {
        self.is_established
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}
