use crate::callbacks::{ConnectionCallback, MessageCallback};
use crate::{EventLoopThreadPool, ReactorSocket, TcpConnection};
use log::{error, trace};
use mio::Interest;
use mio::net::{TcpListener, TcpStream};

pub struct Acceptor {
    listener: TcpListener,
    event_loop_thread_pool: EventLoopThreadPool<TcpConnection>,
    connection_callback: ConnectionCallback,
    message_callback: MessageCallback,
    poll_token: Option<mio::Token>,
}

impl Acceptor {
    pub fn new(
        addr: String,
        event_loop_thread_pool: EventLoopThreadPool<TcpConnection>,
        connection_callback: ConnectionCallback,
        message_callback: MessageCallback,
    ) -> Self {
        Acceptor {
            listener: TcpListener::bind(addr.parse().unwrap()).unwrap(),
            event_loop_thread_pool,
            connection_callback,
            message_callback,
            poll_token: None,
        }
    }

    pub fn on_new_connection(&mut self, stream: TcpStream) {
        let reactor_index = self.event_loop_thread_pool.reactor_index();
        let reactor = self.event_loop_thread_pool.get_next_reactor();
        trace!(
            "New connection [{}->{}] will send to reactor({})",
            stream.local_addr().unwrap(),
            stream.peer_addr().unwrap(),
            reactor_index
        );
        let connection = TcpConnection::new(
            stream,
            self.connection_callback.clone(),
            self.message_callback.clone(),
            Interest::READABLE,
            reactor.get_sender(),
            reactor.get_waker(),
        );
        reactor.register(connection);
        trace!(
            "Connection registered with reactor: {}",
            self.event_loop_thread_pool.reactor_index()
        );
    }
}

impl ReactorSocket for Acceptor {
    type Socket = TcpListener;

    fn handle_event(&mut self, event: &mio::event::Event, _receive_time: std::time::Instant) {
        if event.is_readable() {
            loop {
                match self.listener.accept() {
                    Ok((stream, _)) => {
                        self.on_new_connection(stream);
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // 没有更多连接等待，退出内层循环
                        break;
                    }
                    Err(e) => {
                        error!("Failed to accept connection: {}", e);
                    }
                }
            }
        } else {
            error!("Unexpected event for Acceptor: {:?}", event);
        }
    }

    fn socket(&mut self) -> &mut Self::Socket {
        &mut self.listener
    }

    fn interest(&self) -> mio::Interest {
        mio::Interest::READABLE
    }

    fn set_interest(&mut self, _interest: mio::Interest) {
        error!("Acceptor just read")
    }

    fn handle_connection(&self, is_connected: bool) {
        trace!(
            "Acceptor connection status: {}",
            if is_connected { "ON" } else { "OFF" }
        );
    }

    fn write(&mut self, _data: &[u8]) -> std::io::Result<usize> {
        error!("Acceptor does not support write operation");
        Err(std::io::ErrorKind::Other.into())
    }

    fn stash_output(&mut self, _data: &[u8]) {
        error!("Acceptor does not support output stashing");
    }

    fn poll_token(&self) -> Option<mio::Token> {
        self.poll_token
    }

    fn set_poll_token(&mut self, token: mio::Token) {
        self.poll_token = Some(token);
    }

    fn send(&mut self, _addr: std::net::SocketAddr, _data: &[u8]) -> std::io::Result<usize> {
        panic!("Invalid call")
    }
}
