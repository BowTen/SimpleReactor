use std::{net::SocketAddr, sync::Arc};

use crate::{ReactorSocket, TcpConnection, UdpSocket, channel::Sender, reactor::ReactorSignal};

pub struct SocketRemote<S>
where
    S: ReactorSocket,
{
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
    poll_token: mio::Token,
    sender: Sender<ReactorSignal<S>>,
    waker: Arc<mio::Waker>,
}

impl<S> SocketRemote<S>
where
    S: ReactorSocket,
{
    pub fn new(
        local_addr: SocketAddr,
        peer_addr: SocketAddr,
        poll_token: mio::Token,
        sender: Sender<ReactorSignal<S>>,
        waker: Arc<mio::Waker>,
    ) -> Self {
        SocketRemote {
            local_addr,
            peer_addr,
            poll_token,
            sender,
            waker,
        }
    }
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }
    pub fn shutdown(&self) {
        self.sender.send(ReactorSignal::ShutDown(self.poll_token));
        self.waker.wake().expect("Failed to wake reactor");
    }
    pub fn reregister(&self, interest: mio::Interest) {
        self.sender
            .send(ReactorSignal::ReRegister(self.poll_token, interest));
        self.waker.wake().expect("Failed to wake reactor");
    }
}

impl SocketRemote<TcpConnection> {
    pub fn write(&self, data: &[u8]) {
        self.sender
            .send(ReactorSignal::Write(self.poll_token, data.to_vec()));
        self.waker.wake().expect("Failed to wake reactor");
    }
}

impl SocketRemote<UdpSocket> {
    pub fn send(&self, addr: SocketAddr, data: &[u8]) {
        self.sender
            .send(ReactorSignal::Send(self.poll_token, addr, data.to_vec()));
        self.waker.wake().expect("Failed to wake reactor");
    }
}
