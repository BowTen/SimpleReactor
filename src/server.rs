use core::panic;

use log::warn;

use crate::{
    Acceptor, EventLoopThread, EventLoopThreadPool, Reactor, ReactorRemote, ReactorSocket,
    TcpConnection, UdpSocket,
    callbacks::{ConnectionCallback, DatagramCallback, MessageCallback},
    reactor::ReactorSignal,
};

pub struct Server {
    addr: String,
    acceptor_reactor: Option<Reactor<Acceptor>>,
    udp_reactor: Option<Reactor<UdpSocket>>,
    event_loop_thread_pool: Option<EventLoopThreadPool<TcpConnection>>,
    connection_callback: Option<ConnectionCallback>,
    message_callback: Option<MessageCallback>,
    datagram_callback: Option<DatagramCallback>,
    tcp: bool,
    udp: bool,
}

impl Server {
    pub fn tcp_server(
        addr: String,
        mut num_threads: usize,
        message_callback: MessageCallback,
        connection_callback: ConnectionCallback,
    ) -> Self {
        if num_threads == 0 {
            warn!("Number of threads is 0, using 1 instead");
            num_threads = 1;
        }
        let (_, receiver) = crate::reactor_channel::channel::<ReactorSignal<Acceptor>>();
        let reactor = Reactor::new(2, receiver);
        Server {
            addr,
            acceptor_reactor: Some(reactor),
            udp_reactor: None,
            event_loop_thread_pool: Some(EventLoopThreadPool::new(num_threads)),
            connection_callback: Some(connection_callback),
            message_callback: Some(message_callback),
            datagram_callback: None,
            tcp: true,
            udp: false,
        }
    }

    pub fn udp_server(addr: String, datagram_callback: DatagramCallback) -> Self {
        let (_, receiver) = crate::reactor_channel::channel::<ReactorSignal<UdpSocket>>();
        let reactor = Reactor::new(2, receiver);
        Server {
            addr,
            acceptor_reactor: None,
            udp_reactor: Some(reactor),
            event_loop_thread_pool: None,
            connection_callback: None,
            message_callback: None,
            datagram_callback: Some(datagram_callback),
            tcp: false,
            udp: true,
        }
    }

    pub fn tcp_udp_server(
        addr: String,
        mut num_threads: usize,
        message_callback: MessageCallback,
        connection_callback: ConnectionCallback,
        datagram_callback: DatagramCallback,
    ) -> Self {
        if num_threads == 0 {
            warn!("Number of threads is 0, using 1 instead");
            num_threads = 1;
        }
        let (_, acceptor_receiver) = crate::reactor_channel::channel::<ReactorSignal<Acceptor>>();
        let acceptor_reactor = Reactor::new(2, acceptor_receiver);
        let (_, udp_receiver) = crate::reactor_channel::channel::<ReactorSignal<UdpSocket>>();
        let udp_reactor = Reactor::new(2, udp_receiver);
        Server {
            addr,
            acceptor_reactor: Some(acceptor_reactor),
            udp_reactor: Some(udp_reactor),
            event_loop_thread_pool: Some(EventLoopThreadPool::new(num_threads)),
            connection_callback: Some(connection_callback),
            message_callback: Some(message_callback),
            datagram_callback: Some(datagram_callback),
            tcp: true,
            udp: true,
        }
    }

    fn get_acceptor_reactor(mut self) -> Reactor<Acceptor> {
        self.event_loop_thread_pool.as_mut().unwrap().run();
        let acceptor = Acceptor::new(
            self.addr.clone(),
            self.event_loop_thread_pool.take().unwrap(),
            self.connection_callback.unwrap().clone(),
            self.message_callback.unwrap().clone(),
        );
        self.acceptor_reactor.as_mut().unwrap().register(acceptor);
        println!("TCP Server is running on {}", self.addr);
        self.acceptor_reactor.unwrap()
    }

    fn get_udp_reactor(mut self) -> Reactor<UdpSocket> {
        let signal_sender = self.udp_reactor.as_mut().unwrap().get_remote().get_sender();
        let waker = self.udp_reactor.as_mut().unwrap().get_waker();
        let socket = mio::net::UdpSocket::bind(self.addr.parse().unwrap()).unwrap();
        let udp_socket = UdpSocket::new(
            socket,
            self.datagram_callback.as_ref().unwrap().clone(),
            signal_sender,
            waker,
        );
        self.udp_reactor.as_mut().unwrap().register(udp_socket);
        println!("UDP Server is running on {}", self.addr);
        self.udp_reactor.unwrap()
    }

    pub fn run(mut self) {
        if self.tcp && self.udp {
            let signal_sender = self.udp_reactor.as_mut().unwrap().get_remote().get_sender();
            let waker = self.udp_reactor.as_mut().unwrap().get_waker();
            let socket = mio::net::UdpSocket::bind(self.addr.parse().unwrap()).unwrap();
            let udp_socket = UdpSocket::new(
                socket,
                self.datagram_callback.as_ref().unwrap().clone(),
                signal_sender,
                waker,
            );
            self.udp_reactor.as_mut().unwrap().register(udp_socket);
            println!("UDP Server is running on {}", self.addr);
            Self::run_reactor_in_thread(self.udp_reactor.unwrap());

            self.event_loop_thread_pool.as_mut().unwrap().run();
            let acceptor = Acceptor::new(
                self.addr.clone(),
                self.event_loop_thread_pool.take().unwrap(),
                self.connection_callback.unwrap().clone(),
                self.message_callback.unwrap().clone(),
            );
            self.acceptor_reactor.as_mut().unwrap().register(acceptor);
            println!("TCP Server is running on {}", self.addr);
            self.acceptor_reactor.unwrap().run();
        } else if self.tcp {
            self.get_acceptor_reactor().run();
        } else if self.udp {
            self.get_udp_reactor().run();
        } else {
            panic!("Invalid server status, must be tcp or udp or both");
        }
    }

    fn run_reactor_in_thread<S>(reactor: Reactor<S>)
    where
        S: ReactorSocket + 'static,
    {
        let mut event_loop_thread = EventLoopThread::with_reactor(reactor);
        event_loop_thread.run();
    }

    pub fn get_quiter(&self) -> ServerQuiter {
        let acceptor_remote = self.acceptor_reactor.as_ref().map(|r| r.get_remote());
        let udp_remote = self.udp_reactor.as_ref().map(|r| r.get_remote());
        let tcp_remotes = self
            .event_loop_thread_pool
            .as_ref()
            .map(|pool| pool.get_remotes());
        ServerQuiter::new(acceptor_remote, tcp_remotes, udp_remote)
    }
}

pub struct ServerQuiter {
    acceptor_remote: Option<ReactorRemote<Acceptor>>,
    tcp_remotes: Option<Vec<ReactorRemote<TcpConnection>>>,
    udp_remote: Option<ReactorRemote<UdpSocket>>,
}

impl ServerQuiter {
    pub fn new(
        acceptor_remote: Option<ReactorRemote<Acceptor>>,
        tcp_remotes: Option<Vec<ReactorRemote<TcpConnection>>>,
        udp_remote: Option<ReactorRemote<UdpSocket>>,
    ) -> Self {
        ServerQuiter {
            acceptor_remote,
            tcp_remotes,
            udp_remote,
        }
    }

    pub fn quit(&self) {
        if let Some(remote) = &self.acceptor_remote {
            remote.quit();
        }
        if let Some(remotes) = &self.tcp_remotes {
            for remote in remotes {
                remote.quit();
            }
        }
        if let Some(remote) = &self.udp_remote {
            remote.quit();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::Arc,
        thread::{self, sleep},
        time::Duration,
    };

    use log::info;

    use crate::{Server, TcpConnection};

    fn message_callback(
        remote: Arc<crate::SocketRemote<TcpConnection>>,
        buffer: &mut crate::Buffer,
        _receive_time: std::time::Instant,
    ) {
        info!(
            "Received message from {}: {}",
            remote.peer_addr(),
            buffer.retrieve_all_as_string()
        );
    }

    fn connection_callback(remote: Arc<crate::SocketRemote<TcpConnection>>, is_connected: bool) {
        if is_connected {
            info!("New connection established: {}", remote.peer_addr());
        } else {
            info!("Connection closed: {}", remote.peer_addr());
        }
    }

    fn datagram_callback(
        _remote: Arc<crate::SocketRemote<crate::UdpSocket>>,
        buffer: &mut [u8],
        addr: std::net::SocketAddr,
        _receive_time: std::time::Instant,
    ) {
        info!(
            "Received datagram from {}: {}",
            addr,
            String::from_utf8_lossy(buffer)
        );
    }

    fn run_tcp_server_one_sec_and_quit() {
        let addr = "127.0.0.1:8888".to_string();
        let server = Server::tcp_server(
            addr.clone(),
            4,
            Arc::new(message_callback),
            Arc::new(connection_callback),
        );
        let quiter = server.get_quiter();

        let tid = thread::spawn(move || {
            sleep(Duration::from_millis(1000));
            quiter.quit();
        });
        server.run();

        tid.join().expect("fail to wait thread");
    }

    fn run_udp_server_one_sec_and_quit() {
        let addr = "127.0.0.1:8888".to_string();
        let server = Server::udp_server(addr.clone(), Arc::new(datagram_callback));
        let quiter = server.get_quiter();

        let tid = thread::spawn(move || {
            sleep(Duration::from_millis(1000));
            quiter.quit();
        });
        server.run();

        tid.join().expect("fail to wait thread");
    }

    fn run_tcp_udp_server_one_sec_and_quit() {
        let addr = "127.0.0.1:8888".to_string();
        let server = Server::tcp_udp_server(
            addr.clone(),
            4,
            Arc::new(message_callback),
            Arc::new(connection_callback),
            Arc::new(datagram_callback),
        );
        let quiter = server.get_quiter();

        let tid = thread::spawn(move || {
            sleep(Duration::from_millis(1000));
            quiter.quit();
        });
        server.run();

        tid.join().expect("fail to wait thread");
    }

    #[test]
    fn test_server() {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
        info!("env_logger inited");
        run_tcp_server_one_sec_and_quit();
        run_udp_server_one_sec_and_quit();
        run_tcp_udp_server_one_sec_and_quit();
    }
}
