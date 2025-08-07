use std::{net::SocketAddr, sync::{atomic::AtomicBool, Arc}};

use crate::{ReactorSocket, TcpConnection, UdpSocket, reactor_channel::Sender, reactor::ReactorSignal};

pub struct SocketRemote<S>
where
    S: ReactorSocket,
{
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
    poll_token: mio::Token,
    sender: Sender<ReactorSignal<S>>,
    waker: Arc<mio::Waker>,
    is_established: Arc<AtomicBool>,
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
        is_established: Arc<AtomicBool>,
    ) -> Self {
        SocketRemote {
            local_addr,
            peer_addr,
            poll_token,
            sender,
            waker,
            is_established,
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

    pub fn is_established(&self) -> bool {
        self.is_established.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl SocketRemote<TcpConnection> {
    pub fn write(&self, data: &[u8]) -> bool {
        if !self.is_established() {
            return false;
        }
        self.sender
            .send(ReactorSignal::Write(self.poll_token, data.to_vec()));
        self.waker.wake().expect("Failed to wake reactor");
        true
    }
}

impl SocketRemote<UdpSocket> {
    pub fn send(&self, addr: SocketAddr, data: &[u8]) -> bool {
        if !self.is_established() {
            return false;
        }
        self.sender
            .send(ReactorSignal::Send(self.poll_token, addr, data.to_vec()));
        self.waker.wake().expect("Failed to wake reactor");
        true
    }
}
