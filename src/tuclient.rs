use std::net::SocketAddr;

use crate::callbacks::{ConnectionCallback, DatagramCallback, MessageCallback};
use crate::reactor_channel::channel;
use crate::{EventLoopThread, Reactor, ReactorSocket, SocketRemote, TcpConnection, UdpSocket};

pub struct Client<S>
where
    S: ReactorSocket + 'static,
{
    event_loop_thread: EventLoopThread<S>,
    remote: SocketRemote<S>,
}

impl<S> Client<S>
where
    S: ReactorSocket + 'static,
{
    pub fn listen(&mut self) {
        self.event_loop_thread.run();
    }

    pub fn shutdown(self) {
        self.event_loop_thread.quit();
    }
}

impl Client<UdpSocket> {
    pub fn new(udp_socket: mio::net::UdpSocket, datagram_callback: DatagramCallback) -> Self {
        let local_addr = udp_socket.local_addr().unwrap();
        let (sender, receiver) = channel();
        let mut reactor = Reactor::<UdpSocket>::new(2, receiver);
        let waker = reactor.get_waker();
        let socket = UdpSocket::new(udp_socket, datagram_callback, sender.clone(), waker.clone());
        let socket_status = socket.is_established.clone();
        let token = reactor.register(socket).unwrap();
        let event_loop_thread = EventLoopThread::with_reactor(reactor);
        Self {
            event_loop_thread,
            remote: SocketRemote::new(
                local_addr,
                SocketAddr::from(([0, 0, 0, 0], 0)),
                token,
                sender,
                waker,
                socket_status,
            ),
        }
    }

    pub fn send(&self, addr: SocketAddr, data: &[u8]) -> bool {
        self.remote.send(addr, data)
    }
}

impl Client<TcpConnection> {
    pub fn new(
        addr: String,
        message_callback: MessageCallback,
        connection_callback: ConnectionCallback,
    ) -> Self {
        let (sender, receiver) = channel();
        let mut reactor = Reactor::<TcpConnection>::new(2, receiver);
        let waker = reactor.get_waker();
        let stream = mio::net::TcpStream::connect(addr.parse().unwrap()).unwrap();
        let local_addr = stream.local_addr().unwrap();
        let peer_addr = stream.peer_addr().unwrap();
        let socket = TcpConnection::new(
            stream,
            connection_callback,
            message_callback,
            mio::Interest::READABLE,
            sender.clone(),
            waker.clone(),
        );
        let socket_status = socket.is_established.clone();
        let token = reactor.register(socket).unwrap();
        let event_loop_thread = EventLoopThread::with_reactor(reactor);
        Self {
            event_loop_thread,
            remote: SocketRemote::new(local_addr, peer_addr, token, sender, waker, socket_status),
        }
    }

    pub fn write(&self, data: &[u8]) -> bool {
        self.remote.write(data)
    }
}
